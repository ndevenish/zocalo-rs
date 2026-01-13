use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SlurmConfig {
    pub url: String,
    pub user_token: Option<String>,
    pub user: Option<String>,
    pub api_version: String,
}
