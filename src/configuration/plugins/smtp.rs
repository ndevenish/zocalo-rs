use serde::Deserialize;

use crate::{ConfigError, Configuration, configuration::ExtractConfig};

#[derive(Debug, Clone, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    #[serde(rename = "from")]
    pub from_address: Option<String>,
}

impl ExtractConfig for SmtpConfig {
    type Config = Self;

    fn from_configuration(
        configuration: &Configuration,
    ) -> Result<Option<Self::Config>, ConfigError> {
        match configuration.get_plugins_of_kind("smtp").last() {
            None => Ok(None),
            Some(&plugin) => {
                serde_yaml::from_value(plugin.values.clone()).map_err(ConfigError::YamlError)
            }
        }
    }
}
