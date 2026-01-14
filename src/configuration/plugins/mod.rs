pub mod graylog;
pub mod jmx;
pub mod logging;
pub mod rabbitmq;
pub mod rabbitmqapi;
pub mod slurm;
pub mod smtp;
pub mod storage;
pub mod transport;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

pub use graylog::{GraylogConfig, GraylogProtocol};
pub use jmx::JmxConfig;
pub use logging::LoggingConfig;
pub use rabbitmq::RabbitMQConfig;
pub use rabbitmqapi::RabbitMQApiConfig;
pub use slurm::SlurmConfig;
pub use smtp::SmtpConfig;
pub use storage::StorageConfig;
pub use transport::TransportConfig;

#[derive(Debug, Clone)]
pub enum PluginConfig {
    Graylog(GraylogConfig),
    Logging(LoggingConfig),
    Storage(StorageConfig),
    Transport(TransportConfig),
    Slurm(SlurmConfig),
    RabbitMQ(RabbitMQConfig),
    RabbitMQApi(RabbitMQApiConfig),
    Smtp(SmtpConfig),
    Jmx(JmxConfig),
}

#[derive(Debug, Clone)]
pub enum PluginDefinition {
    Resolved(PluginConfig),
    Unresolved(PathBuf),
}

impl PluginDefinition {
    pub fn is_resolved(&self) -> bool {
        matches!(self, PluginDefinition::Resolved(_))
    }

    pub fn as_config(&self) -> Option<&PluginConfig> {
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

fn hashmap_to_yaml_value(map: HashMap<String, serde_yaml::Value>) -> serde_yaml::Value {
    let mapping: serde_yaml::Mapping = map
        .into_iter()
        .map(|(k, v)| (serde_yaml::Value::String(k), v))
        .collect();
    serde_yaml::Value::Mapping(mapping)
}

impl<'de> Deserialize<'de> for PluginConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PluginConfigVisitor;

        impl<'de> Visitor<'de> for PluginConfigVisitor {
            type Value = PluginConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map with a 'plugin' key")
            }

            fn visit_map<M>(self, mut map: M) -> Result<PluginConfig, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut plugin_type: Option<String> = None;
                let mut values: HashMap<String, serde_yaml::Value> = HashMap::new();

                while let Some(key) = map.next_key::<String>()? {
                    if key == "plugin" {
                        plugin_type = Some(map.next_value()?);
                    } else {
                        values.insert(key, map.next_value()?);
                    }
                }

                let plugin_type = plugin_type.ok_or_else(|| de::Error::missing_field("plugin"))?;

                match plugin_type.as_str() {
                    "graylog" => {
                        let config: GraylogConfig =
                            serde_yaml::from_value(hashmap_to_yaml_value(values))
                                .map_err(de::Error::custom)?;
                        Ok(PluginConfig::Graylog(config))
                    }
                    "logging" => {
                        let config: LoggingConfig =
                            serde_yaml::from_value(hashmap_to_yaml_value(values))
                                .map_err(de::Error::custom)?;
                        Ok(PluginConfig::Logging(config))
                    }
                    "storage" => Ok(PluginConfig::Storage(StorageConfig { values })),
                    "transport" => {
                        let config: TransportConfig =
                            serde_yaml::from_value(hashmap_to_yaml_value(values))
                                .map_err(de::Error::custom)?;
                        Ok(PluginConfig::Transport(config))
                    }
                    "slurm" => {
                        let config: SlurmConfig =
                            serde_yaml::from_value(hashmap_to_yaml_value(values))
                                .map_err(de::Error::custom)?;
                        Ok(PluginConfig::Slurm(config))
                    }
                    "rabbitmq" => {
                        let config: RabbitMQConfig =
                            serde_yaml::from_value(hashmap_to_yaml_value(values))
                                .map_err(de::Error::custom)?;
                        Ok(PluginConfig::RabbitMQ(config))
                    }
                    "rabbitmqapi" => {
                        let config: RabbitMQApiConfig =
                            serde_yaml::from_value(hashmap_to_yaml_value(values))
                                .map_err(de::Error::custom)?;
                        Ok(PluginConfig::RabbitMQApi(config))
                    }
                    "smtp" => {
                        let config: SmtpConfig =
                            serde_yaml::from_value(hashmap_to_yaml_value(values))
                                .map_err(de::Error::custom)?;
                        Ok(PluginConfig::Smtp(config))
                    }
                    "jmx" => {
                        let config: JmxConfig =
                            serde_yaml::from_value(hashmap_to_yaml_value(values))
                                .map_err(de::Error::custom)?;
                        Ok(PluginConfig::Jmx(config))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &[
                            "graylog",
                            "jmx",
                            "logging",
                            "rabbitmq",
                            "rabbitmqapi",
                            "slurm",
                            "smtp",
                            "storage",
                            "transport",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_map(PluginConfigVisitor)
    }
}
