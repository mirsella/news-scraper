use serde::Deserialize;
use std::{fs::read_to_string, path::PathBuf};

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

pub fn load_config(path: Option<&str>) -> Config {
    let path = path.unwrap_or("config.toml");
    toml::from_str(
        read_to_string(path)
            .unwrap_or_else(|err| panic!("{}: {}", path, err))
            .as_str(),
    )
    .unwrap()
}
