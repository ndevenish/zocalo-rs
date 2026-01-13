use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TransportConfig {
    pub default: String,
}
