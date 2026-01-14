pub mod environment;
pub mod format_spec;
pub mod plugins;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use thiserror::Error;

pub use environment::Environment;
pub use plugins::{
    GraylogConfig, JmxConfig, LoggingConfig, PluginConfig, PluginDefinition, RabbitMQApiConfig,
    RabbitMQConfig, SlurmConfig, SmtpConfig, TransportConfig, UnknownConfig,
};

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

/// A consolidated view of all plugins from activated environments.
///
/// Each plugin type has an `Option` field that contains the last activated
/// plugin of that type. Storage plugins are merged into a single lookup table.
#[derive(Debug, Clone, Default)]
pub struct Configuration {
    /// The activated environment names, if any.
    pub environments: Vec<String>,
    /// Graylog configuration.
    pub graylog: Option<GraylogConfig>,
    /// JMX configuration.
    pub jmx: Option<JmxConfig>,
    /// Logging configuration.
    pub logging: Option<LoggingConfig>,
    /// RabbitMQ AMQP connection configuration.
    pub rabbitmq: Option<RabbitMQConfig>,
    /// RabbitMQ HTTP API configuration.
    pub rabbitmqapi: Option<RabbitMQApiConfig>,
    /// Slurm configuration.
    pub slurm: Option<SlurmConfig>,
    /// SMTP configuration.
    pub smtp: Option<SmtpConfig>,
    /// Transport configuration.
    pub transport: Option<TransportConfig>,
    /// Merged storage values from all storage plugins.
    pub storage: HashMap<String, serde_yaml::Value>,
    /// Any unknown plugins
    pub unknown: Vec<UnknownConfig>,
}

#[derive(Debug)]
pub struct ConfigurationManager {
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
impl ConfigurationManager {
    /// Load configuration from the `ZOCALO_CONFIG` environment variable.
    ///
    /// Returns an empty configuration if the variable is not set.
    pub fn from_env() -> Result<Self, ConfigError> {
        match std::env::var(ZOCALO_CONFIG_ENV) {
            Ok(path) => Self::from_file(path),
            Err(_) => Ok(Self::empty()),
        }
    }

    /// Create an empty configuration with no environments or plugins.
    pub fn empty() -> Self {
        ConfigurationManager {
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

        let mut config = ConfigurationManager {
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

    pub fn plugin_definitions(&self) -> impl Iterator<Item = (&str, &PluginDefinition)> {
        self.plugin_definitions.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn get_plugin(&self, name: &str) -> Option<&PluginDefinition> {
        self.plugin_definitions.get(name)
    }

    pub fn resolve_plugin(&mut self, name: &str) -> Result<&PluginConfig, ConfigError> {
        if !self.plugin_definitions.contains_key(name) {
            return Err(ConfigError::UndefinedPlugin(name.to_string()));
        }

        // Check if already resolved
        if let Some(PluginDefinition::Resolved(config)) = self.plugin_definitions.get(name) {
            return Ok(unsafe {
                // SAFETY: We're returning a reference that will live as long as self
                &*(config as *const PluginConfig)
            });
        }

        // Need to resolve from file
        let path = match self.plugin_definitions.get(name) {
            Some(PluginDefinition::Unresolved(path)) => path.clone(),
            _ => unreachable!(),
        };

        let content = fs::read_to_string(&path).map_err(|e| {
            ConfigError::PluginResolutionError(name.to_string(), format!("{:?}: {e}", &path))
        })?;

        let plugin_config: PluginConfig = serde_yaml::from_str(&content)
            .map_err(|e| ConfigError::PluginResolutionError(name.to_string(), e.to_string()))?;

        self.plugin_definitions
            .insert(name.to_string(), PluginDefinition::Resolved(plugin_config));

        match self.plugin_definitions.get(name) {
            Some(PluginDefinition::Resolved(config)) => Ok(config),
            _ => unreachable!(),
        }
    }

    pub fn activate_environment(&mut self, name: &str) -> Result<(), ConfigError> {
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

    /// Activate environments.
    ///
    /// If no environments are specified, falls back to:
    /// 1. The `ZOCALO_DEFAULT_ENV` environment variable
    /// 2. The "default" environment (if defined)
    pub fn activate<T: AsRef<str> + std::fmt::Display>(
        &mut self,
        envs: Option<&[T]>,
    ) -> Result<Vec<String>, ConfigError> {
        let envs_to_activate: Vec<String> = match envs {
            Some(e) if !e.is_empty() => e.iter().map(|s| s.to_string()).collect(),
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

    /// Resolve a unified view of all active environment settings
    ///
    /// Returns an `ActivatedEnvironment` containing all resolved plugin
    /// configurations. For most plugin types, the last activated plugin wins.
    ///
    /// Storage plugins are merged into a single lookup table.
    pub fn resolve(&mut self) -> Result<Configuration, ConfigError> {
        let mut result = Configuration {
            environments: self.activated.clone(),
            ..Default::default()
        };
        // Go through every active environment
        for env_name in self.activated.clone() {
            // self.activate_environment(env_name)?;

            // Collect plugin names first to avoid borrow issues
            let plugin_names: Vec<String> = self
                .environments
                .get(&env_name)
                .unwrap()
                .all_plugins()
                .map(|s| s.to_string())
                .collect();

            for plugin_name in plugin_names {
                let plugin = self.resolve_plugin(&plugin_name)?;
                match plugin {
                    PluginConfig::Graylog(c) => result.graylog = Some(c.clone()),
                    PluginConfig::Jmx(c) => result.jmx = Some(c.clone()),
                    PluginConfig::Logging(c) => result.logging = Some(c.clone()),
                    PluginConfig::RabbitMQ(c) => result.rabbitmq = Some(c.clone()),
                    PluginConfig::RabbitMQApi(c) => result.rabbitmqapi = Some(c.clone()),
                    PluginConfig::Slurm(c) => result.slurm = Some(c.clone()),
                    PluginConfig::Smtp(c) => result.smtp = Some(c.clone()),
                    PluginConfig::Storage(c) => {
                        result.storage.extend(c.values.clone());
                    }
                    PluginConfig::Transport(c) => result.transport = Some(c.clone()),
                    PluginConfig::Unknown(c) => result.unknown.push(c.clone()),
                }
            }
        }

        Ok(result)
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
    Inline(PluginConfig),
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
                let config = PluginConfig::deserialize(de::value::MapAccessDeserializer::new(map))?;
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
        let config = ConfigurationManager::from_string(SAMPLE_CONFIG).unwrap();
        assert_eq!(config.version(), 1);
        println!("{config:#?}");
    }

    #[test]
    fn test_environments() {
        let config = ConfigurationManager::from_string(SAMPLE_CONFIG).unwrap();
        let envs: Vec<_> = config.environments().collect();
        assert!(envs.contains(&"live"));
        assert!(envs.contains(&"dev"));
        assert!(envs.contains(&"default"));
    }

    #[test]
    fn test_plugin_definitions() {
        let config = ConfigurationManager::from_string(SAMPLE_CONFIG).unwrap();

        // Check inline plugin
        let graylog = config.get_plugin("graylog-basic").unwrap();
        assert!(graylog.is_resolved());

        // Check external file reference
        let external = config.get_plugin("external-config").unwrap();
        assert!(!external.is_resolved());
    }

    #[test]
    fn test_graylog_plugin() {
        let config = ConfigurationManager::from_string(SAMPLE_CONFIG).unwrap();
        if let Some(PluginDefinition::Resolved(PluginConfig::Graylog(graylog))) =
            config.get_plugin("graylog-basic")
        {
            assert_eq!(graylog.host, "graylog.example.com");
            assert_eq!(graylog.port, 12201);
            assert_eq!(graylog.protocol, plugins::GraylogProtocol::Udp);
        } else {
            panic!("Expected Graylog plugin");
        }
    }

    #[test]
    fn test_storage_plugin() {
        let config = ConfigurationManager::from_string(SAMPLE_CONFIG).unwrap();
        if let Some(PluginDefinition::Resolved(PluginConfig::Storage(storage))) =
            config.get_plugin("storage-config")
        {
            assert!(storage.values.contains_key("zocalo.recipe_directory"));
        } else {
            panic!("Expected Storage plugin");
        }
    }

    #[test]
    fn test_unsupported_version() {
        let config_str = "version: 2\nenvironments: {}";
        let result = ConfigurationManager::from_string(config_str);
        assert!(matches!(result, Err(ConfigError::UnsupportedVersion(2))));
    }

    #[test]
    fn test_parse_sample_config() {
        let config = ConfigurationManager::from_string(SAMPLE_CONFIG).unwrap();

        // Check version
        assert_eq!(config.version(), 1);

        // Check environments exist
        let envs: Vec<_> = config.environments().collect();
        assert!(envs.contains(&"live"));
        assert!(envs.contains(&"dev"));
        assert!(envs.contains(&"default"));

        // Check graylog-basic plugin
        if let Some(PluginDefinition::Resolved(PluginConfig::Graylog(graylog))) =
            config.get_plugin("graylog-basic")
        {
            assert_eq!(graylog.host, "graylog.example.com");
            assert_eq!(graylog.port, 12201);
            assert_eq!(graylog.protocol, plugins::GraylogProtocol::Udp);
        } else {
            panic!("Expected graylog-basic plugin");
        }

        // Check transport plugin
        if let Some(PluginDefinition::Resolved(PluginConfig::Transport(transport))) =
            config.get_plugin("transport-config")
        {
            assert_eq!(transport.default, "PikaTransport");
        } else {
            panic!("Expected transport-config plugin");
        }

        // Check storage plugin
        if let Some(PluginDefinition::Resolved(PluginConfig::Storage(storage))) =
            config.get_plugin("storage-config")
        {
            assert!(storage.values.contains_key("zocalo.recipe_directory"));
        } else {
            panic!("Expected storage-config plugin");
        }

        // Check external file references
        let external = config.get_plugin("external-config").unwrap();
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
        assert!(live_env.logging_plugins().is_some());
        assert!(live_env.get_group("plugins").is_some());

        // Check environment alias
        let default_env = config.get_environment("default").unwrap();
        assert_eq!(default_env.alias(), Some("live"));
    }

    #[test]
    fn test_logging_plugin() {
        let config = ConfigurationManager::from_string(SAMPLE_CONFIG).unwrap();

        if let Some(PluginDefinition::Resolved(PluginConfig::Logging(logging))) =
            config.get_plugin("logging-production")
        {
            // Check root logger
            assert!(logging.root.is_some());
            let root = logging.root.as_ref().unwrap();
            assert_eq!(root.level.as_deref(), Some("WARNING"));

            // Check loggers
            assert!(logging.loggers.contains_key("dials"));
            assert!(logging.loggers.contains_key("zocalo"));

            // Check verbose levels
            assert!(!logging.verbose.is_empty());
        } else {
            panic!("Expected logging-production plugin");
        }
    }

    #[test]
    fn test_activate_returns_consolidated_environment() {
        let mut config = ConfigurationManager::from_string(SAMPLE_CONFIG).unwrap();

        // Activate the "live" environment
        config.activate(Some(&["live"])).unwrap();
        let activated = config.resolve().unwrap();

        // Check environments list
        assert_eq!(activated.environments, vec!["live"]);

        // Check graylog was activated
        assert!(activated.graylog.is_some());
        let graylog = activated.graylog.unwrap();
        assert_eq!(graylog.host, "graylog.example.com");
        assert_eq!(graylog.port, 12201);

        // Check transport was activated
        assert!(activated.transport.is_some());
        let transport = activated.transport.unwrap();
        assert_eq!(transport.default, "PikaTransport");

        // Check storage was merged
        assert!(activated.storage.contains_key("zocalo.recipe_directory"));

        // Check logging was activated
        assert!(activated.logging.is_some());
        let logging = activated.logging.unwrap();
        assert_eq!(logging.root.unwrap().level.as_deref(), Some("WARNING"));

        // Check other plugins are None
        assert!(activated.slurm.is_none());
        assert!(activated.rabbitmqapi.is_none());
        assert!(activated.smtp.is_none());
        assert!(activated.jmx.is_none());
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
        let mut config = ConfigurationManager::from_string(config_str).unwrap();
        config.activate(Some(&["test"])).unwrap();
        let activated = config.resolve().unwrap();

        // Both storage keys should be present
        assert!(activated.storage.contains_key("key.a"));
        assert!(activated.storage.contains_key("key.b"));

        // Shared key should have value from last plugin (storage-b)
        assert_eq!(
            activated.storage.get("key.shared"),
            Some(&serde_yaml::Value::String("from-b".to_string()))
        );
    }
}
