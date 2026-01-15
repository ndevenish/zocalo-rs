use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct UnknownConfig {
    pub plugin: String,
    #[serde(flatten)]
    pub values: HashMap<String, serde_yaml::Value>,
}
