//! Support infrastructure for interacting with
//! [Zocalo](https://github.com/diamondlightsource/python-zocalo) from Rust.
//!
//! Currently supports:
//! - Loading configuration files and extracting data values from them, via
//!   loading a [`Configuration`], or constructing directly with environment
//!   defaults via [`configuration::ExtractConfig`] e.g.
//!
//!   Usage example:
//!
//!   ```no_run
//!   use crate::zocalo::ExtractConfig;
//!   let rmq = zocalo::RabbitMQConfig::from_default_env().unwrap().unwrap();
//!   println!("RabbitMQ hosts: {}", rmq.host.join(", "));
//!   ```
//!
//! See [`ExtractConfig` implementors](configuration::ExtractConfig#implementors)
//! for the list of built-in configurations available.

pub mod configuration;
pub mod workflows;

pub use configuration::{
    ConfigError, Configuration, GraylogConfig, JmxConfig, LoggingConfig, RabbitMQApiConfig,
    RabbitMQConfig, SlurmConfig, SmtpConfig, TransportConfig, ZOCALO_CONFIG_ENV,
    ZOCALO_DEFAULT_ENV,
};
pub use configuration::{ExtractConfig, plugins};
