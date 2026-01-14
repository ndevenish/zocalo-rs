use serde::Deserialize;
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};

/// RabbitMQ AMQP connection configuration.
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
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
