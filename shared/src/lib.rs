use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct News {
    pub provider: String,
    pub time: DateTime<Utc>,
    pub title: String,
    pub description: String,
    pub body: String,
    pub link: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub db_user: String,
    pub db_password: String,
    pub deno_server_url: String,
    pub chrome_headless: Option<bool>,
    pub chrome_concurrent_tabs: Option<usize>,
    pub chrome_data_dir: Option<PathBuf>,
}
impl Config {
    pub fn load(path: &str) -> anyhow::Result<Config> {
        dotenvy::from_filename(path)?;
        let config: Config = envy::from_env()?;
        Ok(config)
    }
}
