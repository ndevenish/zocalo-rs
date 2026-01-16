# Zocalo Compatibility Tools for Rust

[![Crates.io](https://img.shields.io/crates/v/zocalo.svg)](https://crates.io/crates/zocalo)
[![Docs.rs](https://docs.rs/zocalo/badge.svg)](https://docs.rs/zocalo)
[![CI](https://github.com/ndevenish/zocalo-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/ndevenish/zocalo-rs/actions)
[![License](https://img.shields.io/crates/l/zocalo)](https://crates.io/crates/zocalo)

Support infrastructure for interacting with
[Zocalo](https://github.com/diamondlightsource/python-zocalo) systems from
Rust.

Currently supports:
- Loading configuration files and extracting data values from them, via
  a `Configuration` object, or constructing directly with environment
  defaults via the `configuration::ExtractConfig` trait e.g.

Usage example:
```rust
use crate::zocalo::ExtractConfig;

let rmq = zocalo::RabbitMQConfig::from_default_env().unwrap().unwrap();
println!("RabbitMQ hosts: {}", rmq.host.join(", "));
```
