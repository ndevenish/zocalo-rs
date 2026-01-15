pub mod environment;
pub mod format_spec;
pub mod plugins;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use thiserror::Error;

pub use environment::Environment;
pub use plugins::{
    GraylogConfig, JmxConfig, LoggingConfig, PluginDefinition, RabbitMQApiConfig, RabbitMQConfig,
    SlurmConfig, SmtpConfig, TransportConfig, UnknownConfig,
};

use crate::plugins::UnparsedConfig;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse YAML: {0}")]
    YamlError(#[from] serde_yaml::Error),
    #[error("Unsupported configuration version: {0}")]
    UnsupportedVersion(u32),
    #[error("Environment '{0}' is not defined")]
    UndefinedEnvironment(String),
    #[error("Plugin '{0}' is not defined")]
    UndefinedPlugin(String),
    #[error("Failed to resolve plugin '{0}': {1}")]
    PluginResolutionError(String, String),
}

/// Extract a plugin config from a configuration
pub trait ExtractConfig {
    type Config;

    fn from_configuration(
        configuration: &Configuration,
    ) -> Result<Option<Self::Config>, ConfigError>;

    fn from_default_env() -> Result<Option<Self::Config>, ConfigError> {
        let mut conf = Configuration::from_env()?;
        conf.activate(None)?;
        Self::from_configuration(&conf)
    }
}

#[derive(Debug)]
pub struct Configuration {
    version: u32,
    environments: HashMap<String, Environment>,
    plugin_definitions: HashMap<String, PluginDefinition>,
    activated: Vec<String>,
    base_path: PathBuf,
}

/// Environment variable for the default configuration file path.
pub const ZOCALO_CONFIG_ENV: &str = "ZOCALO_CONFIG";

/// Environment variable for the default environment to activate.
pub const ZOCALO_DEFAULT_ENV: &str = "ZOCALO_DEFAULT_ENV";

/// Parse and resolve zocalo configuration files
///
/// See [`format_spec`] for the configuration file format specification,
/// defined by the [original implementation](https://github.com/DiamondLightSource/python-zocalo/tree/main/src/zocalo/configuration).
///
/// Usage to get the current default environment:
/// ```ignore
/// let environment = Configuration::from_env()?.activate(None)?;
/// ```
/// This returns an [`ActivatedEnvironment`], from which individial
/// plugin settings can be read, if present.
impl Configuration {
    /// Load configuration from the `ZOCALO_CONFIG` environment variable.
    ///
    /// Returns an empty configuration if the variable is not set.
    pub fn from_env() -> Result<Self, ConfigError> {
        match std::env::var(ZOCALO_CONFIG_ENV) {
            Ok(path) => Self::from_file(path),
            Err(_) => Ok(Self::empty()),
        }
    }

    pub fn activated_default_env() -> Result<Self, ConfigError> {
        let mut c = Self::from_env()?;
        c.activate(None)?;
        Ok(c)
    }

    /// Create an empty configuration with no environments or plugins.
    pub fn empty() -> Self {
        Configuration {
            version: 1,
            environments: HashMap::new(),
            plugin_definitions: HashMap::new(),
            activated: Vec::new(),
            base_path: std::env::current_dir().unwrap_or_default(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }
        let content = fs::read_to_string(path)?;
        let base_path = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self::from_string_with_base(&content, base_path)
    }

    pub fn from_string(content: &str) -> Result<Self, ConfigError> {
        Self::from_string_with_base(content, std::env::current_dir().unwrap_or_default())
    }

    /// Load a configuration from a String, with a explicit working dir
    fn from_string_with_base(content: &str, base_path: PathBuf) -> Result<Self, ConfigError> {
        let raw: RawConfiguration = serde_yaml::from_str(content)?;

        if raw.version != 1 {
            return Err(ConfigError::UnsupportedVersion(raw.version));
        }

        let mut config = Configuration {
            version: raw.version,
            environments: HashMap::new(),
            plugin_definitions: HashMap::new(),
            activated: Vec::new(),
            base_path,
        };

        // Process environments
        for (name, env_def) in raw.environments {
            let env = Environment::from_raw(env_def)?;
            config.environments.insert(name, env);
        }

        // Process plugin definitions
        for (name, def) in raw.plugin_definitions {
            let plugin_def = match def {
                RawPluginDefinition::Path(path_str) => {
                    let expanded = shellexpand::tilde(&path_str);
                    let path = if Path::new(expanded.as_ref()).is_absolute() {
                        PathBuf::from(expanded.as_ref())
                    } else {
                        config.base_path.join(expanded.as_ref())
                    };
                    PluginDefinition::Unresolved(path)
                }
                RawPluginDefinition::Inline(plugin_config) => {
                    PluginDefinition::Resolved(plugin_config)
                }
            };
            config.plugin_definitions.insert(name, plugin_def);
        }

        // Validate environment references
        for (env_name, env) in &config.environments {
            for plugin_name in env.all_plugins() {
                if !config.plugin_definitions.contains_key(plugin_name) {
                    return Err(ConfigError::Invalid(format!(
                        "Environment '{}' references undefined plugin '{}'",
                        env_name, plugin_name
                    )));
                }
            }
        }

        Ok(config)
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn environments(&self) -> impl Iterator<Item = &str> {
        self.environments.keys().map(|s| s.as_str())
    }

    pub fn get_environment(&self, name: &str) -> Option<&Environment> {
        self.environments.get(name)
    }

    pub fn default_environment(&self) -> Option<&str> {
        self.environments
            .get("default")
            .and_then(|env| env.alias())
            .or_else(|| {
                if self.environments.contains_key("default") {
                    Some("default")
                } else {
                    None
                }
            })
    }

    pub fn activated_environments(&self) -> &[String] {
        &self.activated
    }

    fn resolve_plugin<'b>(&'b mut self, name: &str) -> Result<&'b UnparsedConfig, ConfigError> {
        match self.plugin_definitions.entry(name.to_string()) {
            Entry::Vacant(_) => {
                return Err(ConfigError::UndefinedPlugin(name.to_string()));
            }
            Entry::Occupied(mut entry) => {
                if let PluginDefinition::Unresolved(path) = entry.get() {
                    let path = path.clone();
                    let content = fs::read_to_string(&path).map_err(|e| {
                        ConfigError::PluginResolutionError(
                            name.to_string(),
                            format!("{:?}: {e}", &path),
                        )
                    })?;
                    let config: UnparsedConfig = serde_yaml::from_str(&content).map_err(|e| {
                        ConfigError::PluginResolutionError(name.to_string(), e.to_string())
                    })?;
                    entry.insert(PluginDefinition::Resolved(config));
                }
            }
        }

        // We definitely have this entry now
        match self.plugin_definitions.get(name) {
            Some(PluginDefinition::Resolved(config)) => Ok(config),
            _ => unreachable!(),
        }
    }

    fn activate_environment(&mut self, name: &str) -> Result<(), ConfigError> {
        if !self.environments.contains_key(name) {
            return Err(ConfigError::UndefinedEnvironment(name.to_string()));
        }

        let env = self.environments.get(name).unwrap();
        let plugins: Vec<String> = env.all_plugins().map(|s| s.to_string()).collect();

        for plugin_name in plugins {
            self.resolve_plugin(&plugin_name)?;
        }

        self.activated.push(name.to_string());
        Ok(())
    }

    /// Get all plugins with a given `plugin: <named>`
    pub fn get_plugins_of_kind(&self, named: &str) -> Vec<&UnparsedConfig> {
        let all_names: Vec<_> = self
            .activated_environments()
            .iter()
            .flat_map(|env_name| {
                self.environments
                    .get(env_name)
                    .unwrap()
                    .all_plugins()
                    .map(|s| s.to_string())
            })
            .collect();

        all_names
            .iter()
            .map(|n| self.plugin_definitions.get(n).unwrap().as_config().unwrap())
            .filter(|&u| u.plugin == named)
            .collect()
    }
    pub fn plugin_definitions(&self) -> impl Iterator<Item = (&str, &PluginDefinition)> {
        self.plugin_definitions.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Activate environments.
    ///
    /// If no environments are specified, falls back to:
    /// 1. The `ZOCALO_DEFAULT_ENV` environment variable
    /// 2. The "default" environment (if defined)
    ///
    /// Accepts `None`, `vec!["env1", "env2"]`, or `Some(vec![...])`.
    pub fn activate(
        &mut self,
        envs: impl Into<Option<Vec<String>>>,
    ) -> Result<Vec<String>, ConfigError> {
        let envs_to_activate: Vec<String> = match envs.into() {
            Some(e) if !e.is_empty() => e,
            _ => {
                // Check ZOCALO_DEFAULT_ENV first
                if let Ok(env) = std::env::var(ZOCALO_DEFAULT_ENV) {
                    vec![env]
                } else if let Some(default) = self.default_environment() {
                    vec![default.to_string()]
                } else {
                    vec![]
                }
            }
        };

        for env in &envs_to_activate {
            self.activate_environment(env)?;
        }

        Ok(envs_to_activate)
    }
}

// Raw deserialization types

#[derive(Debug, Deserialize)]
struct RawConfiguration {
    version: u32,
    #[serde(default)]
    environments: HashMap<String, environment::RawEnvironment>,
    #[serde(flatten)]
    plugin_definitions: HashMap<String, RawPluginDefinition>,
}

#[derive(Debug)]
enum RawPluginDefinition {
    Path(String),
    Inline(UnparsedConfig),
}

impl<'de> Deserialize<'de> for RawPluginDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RawPluginDefinitionVisitor;

        impl<'de> Visitor<'de> for RawPluginDefinitionVisitor {
            type Value = RawPluginDefinition;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string path or a map with a 'plugin' key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(RawPluginDefinition::Path(v.to_string()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(RawPluginDefinition::Path(v))
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let config =
                    UnparsedConfig::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(RawPluginDefinition::Inline(config))
            }
        }

        deserializer.deserialize_any(RawPluginDefinitionVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONFIG: &str = r#"
version: 1

graylog-basic:
  plugin: graylog
  protocol: UDP
  host: graylog.example.com
  port: 12201

logging-production:
  plugin: logging
  root:
    level: WARNING
  loggers:
    dials:
      level: INFO
    zocalo:
      level: DEBUG
  verbose:
    - level: INFO
    - level: DEBUG

storage-config:
  plugin: storage
  zocalo.recipe_directory: /path/to/recipes

transport-config:
  plugin: transport
  default: PikaTransport

external-config: /path/to/config.yml

environments:
  default: live

  live:
    logging:
    - graylog-basic
    - logging-production
    plugins:
    - storage-config
    - transport-config

  dev:
    plugins:
    - storage-config
"#;

    #[test]
    fn test_parse_basic_config() {
        let config = Configuration::from_string(SAMPLE_CONFIG).unwrap();
        assert_eq!(config.version(), 1);
        println!("{config:#?}");
    }

    #[test]
    fn test_environments() {
        let config = Configuration::from_string(SAMPLE_CONFIG).unwrap();
        let envs: Vec<_> = config.environments().collect();
        assert!(envs.contains(&"live"));
        assert!(envs.contains(&"dev"));
        assert!(envs.contains(&"default"));
    }

    #[test]
    fn test_plugin_definitions() {
        let config = Configuration::from_string(SAMPLE_CONFIG).unwrap();

        // Check inline plugin
        let graylog = config
            .plugin_definitions()
            .find(|(n, _)| *n == "graylog-basic")
            .map(|(_, d)| d);
        assert!(graylog.is_some());
        assert!(graylog.unwrap().is_resolved());

        // Check external file reference
        let external = config
            .plugin_definitions()
            .find(|(n, _)| *n == "external-config")
            .map(|(_, d)| d);
        assert!(external.is_some());
        assert!(!external.unwrap().is_resolved());
    }

    #[test]
    fn test_graylog_plugin() {
        let mut config = Configuration::from_string(SAMPLE_CONFIG).unwrap();
        config.activate(vec!["live".to_string()]).unwrap();

        let graylog = plugins::GraylogConfig::from_configuration(&config)
            .unwrap()
            .expect("Expected Graylog plugin");

        assert_eq!(graylog.host, "graylog.example.com");
        assert_eq!(graylog.port, 12201);
        assert_eq!(graylog.protocol, plugins::GraylogProtocol::Udp);
    }

    #[test]
    fn test_storage_plugin() {
        let mut config = Configuration::from_string(SAMPLE_CONFIG).unwrap();
        config.activate(vec!["live".to_string()]).unwrap();

        let storage = plugins::StorageConfig::from_configuration(&config)
            .unwrap()
            .expect("Expected Storage plugin");

        assert!(storage.values.contains_key("zocalo.recipe_directory"));
    }

    #[test]
    fn test_unsupported_version() {
        let config_str = "version: 2\nenvironments: {}";
        let result = Configuration::from_string(config_str);
        assert!(matches!(result, Err(ConfigError::UnsupportedVersion(2))));
    }

    #[test]
    fn test_parse_sample_config() {
        let mut config = Configuration::from_string(SAMPLE_CONFIG).unwrap();

        // Check version
        assert_eq!(config.version(), 1);

        // Check environments exist
        let envs: Vec<_> = config.environments().collect();
        assert!(envs.contains(&"live"));
        assert!(envs.contains(&"dev"));
        assert!(envs.contains(&"default"));

        // Activate live environment for typed plugin access
        config.activate(vec!["live".to_string()]).unwrap();

        // Check graylog plugin
        let graylog = plugins::GraylogConfig::from_configuration(&config)
            .unwrap()
            .expect("Expected graylog plugin");
        assert_eq!(graylog.host, "graylog.example.com");
        assert_eq!(graylog.port, 12201);
        assert_eq!(graylog.protocol, plugins::GraylogProtocol::Udp);

        // Check transport plugin
        let transport = plugins::TransportConfig::from_configuration(&config)
            .unwrap()
            .expect("Expected transport plugin");
        assert_eq!(transport.default, "PikaTransport");

        // Check storage plugin
        let storage = plugins::StorageConfig::from_configuration(&config)
            .unwrap()
            .expect("Expected storage plugin");
        assert!(storage.values.contains_key("zocalo.recipe_directory"));

        // Check external file references
        let external = config
            .plugin_definitions()
            .find(|(n, _)| *n == "external-config")
            .map(|(_, d)| d)
            .expect("Expected external-config");
        assert!(!external.is_resolved());
        assert!(
            external
                .as_path()
                .unwrap()
                .to_string_lossy()
                .contains("config.yml")
        );

        // Check live environment structure
        let live_env = config.get_environment("live").unwrap();
        assert!(live_env.get_group("logging").is_some());
        assert!(live_env.get_group("plugins").is_some());

        // Check environment alias
        let default_env = config.get_environment("default").unwrap();
        assert_eq!(default_env.alias(), Some("live"));
    }

    #[test]
    fn test_logging_plugin() {
        let mut config = Configuration::from_string(SAMPLE_CONFIG).unwrap();
        config.activate(vec!["live".to_string()]).unwrap();

        let logging = plugins::LoggingConfig::from_configuration(&config)
            .unwrap()
            .expect("Expected logging plugin");

        // Check root logger
        assert!(logging.root.is_some());
        let root = logging.root.as_ref().unwrap();
        assert_eq!(root.level.as_deref(), Some("WARNING"));

        // Check loggers
        assert!(logging.loggers.contains_key("dials"));
        assert!(logging.loggers.contains_key("zocalo"));

        // Check verbose levels
        assert!(!logging.verbose.is_empty());
    }

    #[test]
    fn test_activate_merges_storage() {
        let config_str = r#"
version: 1

storage-a:
  plugin: storage
  key.a: value-a
  key.shared: from-a

storage-b:
  plugin: storage
  key.b: value-b
  key.shared: from-b

environments:
  test:
    plugins:
    - storage-a
    - storage-b
"#;
        let mut config = Configuration::from_string(config_str).unwrap();
        config.activate(vec!["test".to_string()]).unwrap();

        let storage = plugins::StorageConfig::from_configuration(&config)
            .unwrap()
            .expect("Expected storage plugin");

        // Both storage keys should be present
        assert!(storage.values.contains_key("key.a"));
        assert!(storage.values.contains_key("key.b"));

        // Shared key should have value from last plugin (storage-b)
        assert_eq!(
            storage.values.get("key.shared"),
            Some(&serde_yaml::Value::String("from-b".to_string()))
        );
    }
}
