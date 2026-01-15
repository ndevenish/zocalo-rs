use serde::Deserialize;
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct RabbitMQApiConfig {
    #[serde_as(as = "StringWithSeparator<CommaSeparator, String>")]
    pub base_url: Vec<String>,
    pub username: String,
    pub password: String,
    #[serde(default = "default_vhost")]
    pub vhost: String,
}

fn default_vhost() -> String {
    "/".to_string()
}

impl ExtractConfig for RabbitMQApiConfig {
    type Config = Self;

    fn from_configuration(
        configuration: &Configuration,
    ) -> Result<Option<Self::Config>, ConfigError> {
        match configuration.get_plugins_of_kind("rabbitmqapi").last() {
            None => Ok(None),
            Some(&plugin) => {
                serde_yaml::from_value(plugin.values.clone()).map_err(ConfigError::YamlError)
            }
        }
    }
}
