use core::fmt;
use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub db_user: String,
    pub db_password: String,
    pub openai_api_key: String,
    pub article_parser_url: String,
    pub surrealdb_host: String,
    pub chrome_headless: Option<bool>,
    pub chrome_concurrent: Option<usize>,
    pub chrome_data_dir: Option<PathBuf>,
}
impl Config {
    pub fn load(path: &str) -> Result<Config> {
        dotenvy::from_filename(path)?;
        let config: Config = envy::from_env()?;
        Ok(config)
    }
}
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Config {{\n    db_user: {}\n    db_password: {}\n    openai_api_key: {}\n    article_parser_url: {}\n    surrealdb_url: {}\n    chrome_headless: {:?}\n    chrome_concurrent: {:?}\n    chrome_data_dir: {:?}\n}}",
            "*".repeat(self.db_user.len()),
            "*".repeat(self.db_password.len()),
            "*".repeat(self.openai_api_key.len()),
            self.article_parser_url,
            self.surrealdb_host,
            self.chrome_headless,
            self.chrome_concurrent,
            self.chrome_data_dir,
        )
    }
}
