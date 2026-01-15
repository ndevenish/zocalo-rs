use serde::Deserialize;

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

#[derive(Debug, Clone, Deserialize)]
pub struct JmxConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub username: String,
    pub password: String,
}

impl ExtractConfig for JmxConfig {
    type Config = Self;

    fn from_configuration(
        configuration: &Configuration,
    ) -> Result<Option<Self::Config>, ConfigError> {
        match configuration.get_plugins_of_kind("jmx").last() {
            None => Ok(None),
            Some(&plugin) => {
                serde_yaml::from_value(plugin.values.clone()).map_err(ConfigError::YamlError)
            }
        }
    }
}
