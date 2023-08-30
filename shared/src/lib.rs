use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::{fmt, path::PathBuf};

#[derive(Debug, Default, Clone)]
pub struct News {
    pub provider: String,
    pub time: DateTime<Utc>,
    pub title: String,
    pub description: String,
    pub body: String,
    pub link: String,
}

#[derive(Deserialize)]
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
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Config {{\n    db_user: {}\n    db_password: {}\n    deno_server_url: {}\n    chrome_headless: {:?}\n    chrome_concurrent_tabs: {:?}\n    chrome_data_dir: {:?}\n}}",
            "*".repeat(self.db_user.len()),
            "*".repeat(self.db_password.len()),
            self.deno_server_url,
            self.chrome_headless,
            self.chrome_concurrent_tabs,
            self.chrome_data_dir,
        )
    }
}
