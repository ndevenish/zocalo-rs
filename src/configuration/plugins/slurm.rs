use serde::Deserialize;

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

#[derive(Debug, Clone, Deserialize)]
pub struct SlurmConfig {
    pub url: String,
    pub user_token: Option<String>,
    pub user: Option<String>,
    pub api_version: String,
}

impl ExtractConfig for SlurmConfig {
    type Config = Self;

    fn extract_from(configuration: &Configuration) -> Result<Option<Self::Config>, ConfigError> {
        match configuration.get_plugins_of_kind("slurm").last() {
            None => Ok(None),
            Some(&plugin) => {
                serde_yaml::from_value(plugin.values.clone()).map_err(ConfigError::YamlError)
            }
        }
    }
}
