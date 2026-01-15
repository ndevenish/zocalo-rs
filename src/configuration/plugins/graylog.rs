use serde::Deserialize;

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum GraylogProtocol {
    Udp,
    Tcp,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraylogConfig {
    pub protocol: GraylogProtocol,
    pub host: String,
    pub port: u16,
}

impl ExtractConfig for GraylogConfig {
    type Config = Self;

    fn extract_from(configuration: &Configuration) -> Result<Option<Self::Config>, ConfigError> {
        match configuration.get_plugins_of_kind("graylog").last() {
            None => Ok(None),
            Some(&plugin) => {
                serde_yaml::from_value(plugin.values.clone()).map_err(ConfigError::YamlError)
            }
        }
    }
}
