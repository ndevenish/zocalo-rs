use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct JmxConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub username: String,
    pub password: String,
}
