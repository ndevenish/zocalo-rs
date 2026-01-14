//! # Zocalo Configuration Format
//!
//! The configuration is a YAML file with version 1 that defines **plugin configurations**
//! and **environments**.
//!
//! ## Structure
//!
//! ```yaml
//! version: 1
//!
//! # Plugin definitions (top-level keys other than 'version' and 'environments')
//! plugin-name: <path or inline config>
//!
//! # Environment definitions
//! environments:
//!   env-name: <groups or alias>
//! ```
//!
//! ## Plugin Definitions
//!
//! Plugins can be defined in two ways:
//!
//! **1. External file reference** - a string path to another YAML file:
//! ```yaml
//! slurm: /path/to/slurm-credentials.yml
//! rabbitmq-dev: ~/.zocalo/rabbitmq-credentials.yml  # ~ expansion supported
//! ```
//!
//! **2. Inline configuration** - a map with a required `plugin` key:
//! ```yaml
//! graylog-basic:
//!   plugin: graylog
//!   protocol: UDP
//!   host: graylog.example.com
//!   port: 12201
//! ```
//!
//! ## Plugin Types
//!
//! | Plugin | Required Fields | Optional Fields |
//! |--------|----------------|-----------------|
//! | `graylog` | `protocol` (UDP/TCP), `host`, `port` | |
//! | `logging` | | `handlers`, `root`, `loggers`, `verbose` |
//! | `storage` | | arbitrary key-value pairs |
//! | `transport` | `default` | |
//! | `slurm` | `url`, `api_version` | `user_token`, `user` |
//! | `rabbitmqapi` | `base_url`, `username`, `password` | `vhost` (default: "/") |
//! | `smtp` | `host`, `port` | `from` |
//! | `jmx` | `host`, `port`, `base_url`, `username`, `password` | |
//!
//! ## Environments
//!
//! Environments group plugins together. They can be:
//!
//! **1. Alias** - reference another environment:
//! ```yaml
//! environments:
//!   default: live  # "default" is an alias for "live"
//! ```
//!
//! **2. Grouped definition** - plugins organized by category:
//! ```yaml
//! environments:
//!   live:
//!     logging:
//!       - logging-production
//!     rabbitmq:
//!       - rabbitmq-production
//!     rabbitmqapi:
//!       - rabbitmq-api-production
//!     plugins:
//!       - diamond-zocalo-settings
//!       - mimas-settings
//! ```
//!
//! **3. Simple list** - treated as a `plugins` group:
//! ```yaml
//! environments:
//!   minimal:
//!     - plugin-a
//!     - plugin-b
//! ```
//!
//! ## Plugin Loading Order
//!
//! When an environment is activated, plugins are loaded in this order:
//! 1. Groups sorted alphabetically (e.g., `activemq`, `logging`, `rabbitmq`, `rabbitmqapi`)
//! 2. The `plugins` group always comes last
//!
//! ## External File Format
//!
//! External files must also contain a `plugin` key:
//! ```yaml
//! # slurm-credentials.yml
//! plugin: slurm
//! url: https://slurm.example.com
//! api_version: v0.0.40
//! user_token: secret-token
//! ```

pub mod environment;
pub mod plugins;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use thiserror::Error;

pub use environment::Environment;
pub use plugins::{PluginConfig, PluginDefinition};

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

        let content = fs::read_to_string(&path)
            .map_err(|e| ConfigError::PluginResolutionError(name.to_string(), e.to_string()))?;

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
    pub fn activate(&mut self, envs: Option<&[&str]>) -> Result<Vec<String>, ConfigError> {
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
        let graylog = config.get_plugin("graylog-basic").unwrap();
        assert!(graylog.is_resolved());

        // Check external file reference
        let external = config.get_plugin("external-config").unwrap();
        assert!(!external.is_resolved());
    }

    #[test]
    fn test_graylog_plugin() {
        let config = Configuration::from_string(SAMPLE_CONFIG).unwrap();
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
        let config = Configuration::from_string(SAMPLE_CONFIG).unwrap();
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
        let result = Configuration::from_string(config_str);
        assert!(matches!(result, Err(ConfigError::UnsupportedVersion(2))));
    }

    #[test]
    fn test_parse_sample_yaml() {
        let sample_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample.yaml");
        let config = Configuration::from_file(&sample_path).unwrap();

        // Check version
        assert_eq!(config.version(), 1);

        // Check environments exist
        let envs: Vec<_> = config.environments().collect();
        assert!(envs.contains(&"live"));
        assert!(envs.contains(&"dev_bluesky"));
        assert!(envs.contains(&"devrmq"));
        assert!(envs.contains(&"staging"));
        assert!(envs.contains(&"offline"));

        // Check graylog-basic plugin
        if let Some(PluginDefinition::Resolved(PluginConfig::Graylog(graylog))) =
            config.get_plugin("graylog-basic")
        {
            assert_eq!(graylog.host, "graylog2.diamond.ac.uk");
            assert_eq!(graylog.port, 12201);
            assert_eq!(graylog.protocol, plugins::GraylogProtocol::Udp);
        } else {
            panic!("Expected graylog-basic plugin");
        }

        // Check transport plugin
        if let Some(PluginDefinition::Resolved(PluginConfig::Transport(transport))) =
            config.get_plugin("rabbitmq-default-transport")
        {
            assert_eq!(transport.default, "PikaTransport");
        } else {
            panic!("Expected rabbitmq-default-transport plugin");
        }

        // Check storage plugin with nested structures
        if let Some(PluginDefinition::Resolved(PluginConfig::Storage(storage))) =
            config.get_plugin("diamond-zocalo-settings")
        {
            assert!(storage.values.contains_key("zocalo.recipe_directory"));
            assert!(storage.values.contains_key("zocalo.bridge.queues"));
        } else {
            panic!("Expected diamond-zocalo-settings plugin");
        }

        // Check external file references
        let slurm = config.get_plugin("slurm").unwrap();
        assert!(!slurm.is_resolved());
        assert!(
            slurm
                .as_path()
                .unwrap()
                .to_string_lossy()
                .contains("slurm-credentials.yml")
        );

        // Check live environment structure
        let live_env = config.get_environment("live").unwrap();
        assert!(live_env.logging_plugins().is_some());
        assert!(live_env.rabbitmq_plugins().is_some());
        assert!(live_env.rabbitmqapi_plugins().is_some());

        // Check environment alias
        let default_env = config.get_environment("default").unwrap();
        assert_eq!(default_env.alias(), Some("live"));
    }

    #[test]
    fn test_logging_plugin() {
        let sample_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample.yaml");
        let config = Configuration::from_file(&sample_path).unwrap();

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
}
