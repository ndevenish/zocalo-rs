use serde::Deserialize;
use std::collections::HashMap;

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(transparent)]
pub struct StorageConfig {
    pub values: HashMap<String, serde_yaml::Value>,
}

impl ExtractConfig for StorageConfig {
    type Config = Self;

    fn extract_from(configuration: &Configuration) -> Result<Option<Self::Config>, ConfigError> {
        let plugins = configuration.get_plugins_of_kind("storage");
        if plugins.is_empty() {
            return Ok(None);
        }

        // Merge all storage plugins into one
        let mut merged = HashMap::new();
        for plugin in plugins {
            let config: StorageConfig =
                serde_yaml::from_value(plugin.values.clone()).map_err(ConfigError::YamlError)?;
            merged.extend(config.values);
        }
        Ok(Some(StorageConfig { values: merged }))
    }
}
