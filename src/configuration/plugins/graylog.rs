use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum GraylogProtocol {
    Udp,
    Tcp,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraylogConfig {
    pub protocol: GraylogProtocol,
    pub host: String,
    pub port: u16,
}
