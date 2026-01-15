//! Support infrastructure for interacting with [Zocalo](https://github.com/diamondlightsource/python-zocalo) from Rust.
//!
//! Currently supports:
//! - Loading configuration files and extracting data values from them, via [Configuration].

pub mod configuration;

pub use configuration::plugins;
pub use configuration::{
    ConfigError, Configuration, Environment, GraylogConfig, JmxConfig, LoggingConfig,
    PluginDefinition, RabbitMQApiConfig, RabbitMQConfig, SlurmConfig, SmtpConfig, TransportConfig,
    ZOCALO_CONFIG_ENV, ZOCALO_DEFAULT_ENV,
};
