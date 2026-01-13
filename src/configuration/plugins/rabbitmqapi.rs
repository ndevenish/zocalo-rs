use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RabbitMQApiConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    #[serde(default = "default_vhost")]
    pub vhost: String,
}

fn default_vhost() -> String {
    "/".to_string()
}
