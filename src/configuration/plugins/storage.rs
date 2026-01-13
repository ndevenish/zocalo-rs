use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct StorageConfig {
    pub values: HashMap<String, serde_yaml::Value>,
}
