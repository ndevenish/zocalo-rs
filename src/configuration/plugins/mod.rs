pub mod graylog;
pub mod jmx;
pub mod logging;
pub mod rabbitmq;
pub mod rabbitmqapi;
pub mod slurm;
pub mod smtp;
pub mod storage;
pub mod transport;
pub mod unknown;

use std::path::PathBuf;

use serde::Deserialize;

pub use graylog::{GraylogConfig, GraylogProtocol};
pub use jmx::JmxConfig;
pub use logging::LoggingConfig;
pub use rabbitmq::RabbitMQConfig;
pub use rabbitmqapi::RabbitMQApiConfig;
pub use slurm::SlurmConfig;
pub use smtp::SmtpConfig;
pub use storage::StorageConfig;
pub use transport::TransportConfig;
pub use unknown::UnknownConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct UnparsedConfig {
    pub plugin: String,
    #[serde(flatten)]
    pub values: serde_yaml::Value,
}

#[derive(Debug, Clone)]
pub enum PluginDefinition {
    Resolved(UnparsedConfig),
    Unresolved(PathBuf),
}

impl PluginDefinition {
    pub fn is_resolved(&self) -> bool {
        matches!(self, PluginDefinition::Resolved(_))
    }

    pub fn as_config(&self) -> Option<&UnparsedConfig> {
        match self {
            PluginDefinition::Resolved(config) => Some(config),
            PluginDefinition::Unresolved(_) => None,
        }
    }

    pub fn as_path(&self) -> Option<&PathBuf> {
        match self {
            PluginDefinition::Resolved(_) => None,
            PluginDefinition::Unresolved(path) => Some(path),
        }
    }
}
