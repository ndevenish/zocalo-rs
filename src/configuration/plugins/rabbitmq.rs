use serde::Deserialize;
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

/// RabbitMQ AMQP connection configuration.
#[serde_as]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RabbitMQConfig {
    #[serde_as(as = "StringWithSeparator<CommaSeparator, String>")]
    pub host: Vec<String>,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
    #[serde(default = "default_vhost")]
    pub vhost: String,
}

fn default_vhost() -> String {
    "/".to_string()
}

impl ExtractConfig for RabbitMQConfig {
    type Config = Self;

    fn extract_from(configuration: &Configuration) -> Result<Option<Self::Config>, ConfigError> {
        match configuration.get_plugins_of_kind("pika").last() {
            None => Ok(None),
            Some(&plugin) => {
                serde_yaml::from_value(plugin.values.clone()).map_err(|e| ConfigError::YamlError(e))
            }
        }
    }
}
