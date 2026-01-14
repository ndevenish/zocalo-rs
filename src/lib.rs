pub mod configuration;

pub use configuration::plugins;
pub use configuration::{
    ConfigError, Configuration, Environment, PluginConfig, PluginDefinition,
    ZOCALO_CONFIG_ENV, ZOCALO_DEFAULT_ENV,
};
