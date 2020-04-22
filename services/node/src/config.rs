use config::{Config as ConfigLoader, ConfigError, Environment};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub redis_url: String,
    pub driver: String,
    pub driver_port: u16,
    pub session_id: String,
    pub on_session_create: Option<String>,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = ConfigLoader::new();

        s.set_default("redis_url", "redis://webgrid-redis/")?;
        s.merge(Environment::with_prefix("WEBGRID"))?;

        s.try_into()
    }
}
