//! # Zocalo Configuration Format
//!
//! The configuration is a YAML file with version 1 that defines **plugin configurations**
//! and **environments**.
//!
//! ## Structure
//!
//! ```yaml
//! version: 1
//!
//! # Plugin definitions (top-level keys other than 'version' and 'environments')
//! plugin-name: <path or inline config>
//!
//! # Environment definitions
//! environments:
//!   env-name: <groups or alias>
//! ```
//!
//! ## Plugin Definitions
//!
//! Plugins can be defined in two ways:
//!
//! **1. External file reference** - a string path to another YAML file:
//! ```yaml
//! slurm: /path/to/slurm-credentials.yml
//! rabbitmq-dev: ~/.zocalo/rabbitmq-credentials.yml  # ~ expansion supported
//! ```
//!
//! **2. Inline configuration** - a map with a required `plugin` key:
//! ```yaml
//! graylog-basic:
//!   plugin: graylog
//!   protocol: UDP
//!   host: graylog.example.com
//!   port: 12201
//! ```
//!
//! ## Plugin Types
//!
//! | Plugin | Required Fields | Optional Fields |
//! |--------|----------------|-----------------|
//! | `graylog` | `protocol` (UDP/TCP), `host`, `port` | |
//! | `logging` | | `handlers`, `root`, `loggers`, `verbose` |
//! | `storage` | | arbitrary key-value pairs |
//! | `transport` | `default` | |
//! | `slurm` | `url`, `api_version` | `user_token`, `user` |
//! | `rabbitmqapi` | `base_url`, `username`, `password` | `vhost` (default: "/") |
//! | `smtp` | `host`, `port` | `from` |
//! | `jmx` | `host`, `port`, `base_url`, `username`, `password` | |
//!
//! ## Environments
//!
//! Environments group plugins together. They can be:
//!
//! **1. Alias** - reference another environment:
//! ```yaml
//! environments:
//!   default: live  # "default" is an alias for "live"
//! ```
//!
//! **2. Grouped definition** - plugins organized by category:
//! ```yaml
//! environments:
//!   live:
//!     logging:
//!       - logging-production
//!     rabbitmq:
//!       - rabbitmq-production
//!     rabbitmqapi:
//!       - rabbitmq-api-production
//!     plugins:
//!       - diamond-zocalo-settings
//!       - mimas-settings
//! ```
//!
//! **3. Simple list** - treated as a `plugins` group:
//! ```yaml
//! environments:
//!   minimal:
//!     - plugin-a
//!     - plugin-b
//! ```
//!
//! ## Plugin Loading Order
//!
//! When an environment is activated, plugins are loaded in this order:
//! 1. Groups sorted alphabetically (e.g., `activemq`, `logging`, `rabbitmq`, `rabbitmqapi`)
//! 2. The `plugins` group always comes last
//!
//! ## External File Format
//!
//! External files must also contain a `plugin` key:
//! ```yaml
//! # slurm-credentials.yml
//! plugin: slurm
//! url: https://slurm.example.com
//! api_version: v0.0.40
//! user_token: secret-token
//! ```
