use core::fmt;
use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub struct Config {
    pub db_user: String,
    pub db_password: String,
    pub openai_api_key: String,
    pub article_parser_url: String,
    pub surrealdb_host: String,
    pub rating_chat_prompt: String,
    pub parallel_rating: usize,
    pub chrome_concurrent: Option<usize>,
    pub chrome_data_dir: Option<PathBuf>,
    pub chrome_headless: Option<bool>,
}
impl Config {
    pub fn load(path: &str) -> Result<Config> {
        dotenvy::from_filename(path).context("dotenvy")?;
        if env::var("openai_api_key").is_ok() && env::var("OPENAI_API_KEY").is_ok() {
            env::remove_var("OPENAI_API_KEY");
        }
        let config: Config = envy::from_env().context("envy")?;
        Ok(config)
    }
}
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Config {{\n    db_user: {}\n    db_password: {}\n    openai_api_key: {}\n    article_parser_url: {}\n    surrealdb_url: {}\n    rating_chat_prompt: {:20?}...\n    chrome_concurrent: {:?}\n    chrome_data_dir: {:?}\n}}",
            "*".repeat(self.db_user.len()),
            "*".repeat(self.db_password.len()),
            "*".repeat(self.openai_api_key.len()),
            self.article_parser_url,
            self.surrealdb_host,
            self.rating_chat_prompt,
            self.chrome_concurrent,
            self.chrome_data_dir,
        )
    }
}
