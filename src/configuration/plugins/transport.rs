use serde::Deserialize;

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

#[derive(Debug, Clone, Deserialize)]
pub struct TransportConfig {
    pub default: String,
}

impl ExtractConfig for TransportConfig {
    type Config = Self;

    fn from_configuration(
        configuration: &Configuration,
    ) -> Result<Option<Self::Config>, ConfigError> {
        match configuration.get_plugins_of_kind("transport").last() {
            None => Ok(None),
            Some(&plugin) => {
                serde_yaml::from_value(plugin.values.clone()).map_err(ConfigError::YamlError)
            }
        }
    }
}
