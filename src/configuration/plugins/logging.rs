use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct LoggerConfig {
    pub level: Option<String>,
    #[serde(default)]
    pub handlers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RootLoggerConfig {
    pub level: Option<String>,
    #[serde(default)]
    pub handlers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VerbosityLevel {
    #[serde(default)]
    pub loggers: HashMap<String, LoggerConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default)]
    pub handlers: HashMap<String, serde_yaml::Value>,
    pub root: Option<RootLoggerConfig>,
    #[serde(default)]
    pub loggers: HashMap<String, LoggerConfig>,
    #[serde(default)]
    pub verbose: Vec<VerbosityLevel>,
}
