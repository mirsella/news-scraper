use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::{fs::read_to_string, path::PathBuf};

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
    pub db: DbConfig,
    pub chrome: ChromeConfig,
}

#[derive(Deserialize, Debug)]
pub struct DbConfig {
    pub host: String,
    pub user: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct ChromeConfig {
    #[serde(default)]
    pub headless: Option<bool>,
    #[serde(default)]
    pub concurrent_tabs: Option<usize>,
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
}

pub fn load_config(path: &str) -> anyhow::Result<Config> {
    Ok(toml::from_str(read_to_string(path)?.as_str())?)
}
