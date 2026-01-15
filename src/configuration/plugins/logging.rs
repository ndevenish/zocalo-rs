use serde::Deserialize;
use std::collections::HashMap;

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

#[derive(Debug, Clone, Deserialize)]
pub struct LoggerConfig {
    pub level: Option<String>,
    #[serde(default)]
    pub handlers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RootLoggerConfig {
    pub level: Option<String>,
    #[serde(default)]
    pub handlers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VerbosityLevel {
    #[serde(default)]
    pub loggers: HashMap<String, LoggerConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default)]
    pub handlers: HashMap<String, serde_yaml::Value>,
    pub root: Option<RootLoggerConfig>,
    #[serde(default)]
    pub loggers: HashMap<String, LoggerConfig>,
    #[serde(default)]
    pub verbose: Vec<VerbosityLevel>,
}

impl ExtractConfig for LoggingConfig {
    type Config = Self;

    fn extract_from(configuration: &Configuration) -> Result<Option<Self::Config>, ConfigError> {
        match configuration.get_plugins_of_kind("logging").last() {
            None => Ok(None),
            Some(&plugin) => {
                serde_yaml::from_value(plugin.values.clone()).map_err(ConfigError::YamlError)
            }
        }
    }
}
