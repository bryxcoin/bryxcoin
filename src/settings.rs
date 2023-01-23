use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub port: u16,
    pub public_key: String,
    pub private_key: String,
    pub ledger_repo: String,
    pub mongo_connection_string: String,
    pub mongo_user_database: String,
    pub mongo_user_collection: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let builder = Config::builder();
        let builder = builder.add_source(File::with_name("/etc/bryxcoin/bryxcoin.toml"));

        let config = builder.build()?;

        config.try_deserialize()
    }
}
