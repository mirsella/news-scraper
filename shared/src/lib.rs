use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt, path::PathBuf};

#[derive(Debug, Clone)]
pub struct News {
    pub provider: String,
    pub date: DateTime<Utc>,
    pub title: String,
    pub caption: String,
    pub body: String,
    pub link: String,
}
impl Default for News {
    fn default() -> Self {
        News {
            provider: "DefaultProvider".to_string(),
            date: Utc::now(), // use the current time as default
            title: "DefaultTitle".to_string(),
            caption: "DefaultCaption".to_string(),
            body: "DefaultBody".to_string(),
            link: "http://example.com".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DbNews {
    pub id: Option<surrealdb::opt::RecordId>,
    pub body: Cow<'static, str>,
    pub caption: Cow<'static, str>,
    pub date: surrealdb::sql::Datetime,
    pub link: Cow<'static, str>,
    pub note: Cow<'static, str>,
    pub provider: Cow<'static, str>,
    pub rating: Option<i64>,
    pub tags: Option<Vec<Cow<'static, str>>>,
    pub title: Cow<'static, str>,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub db_user: String,
    pub db_password: String,
    pub deno_server_url: String,
    pub surrealdb_host: String,
    pub chrome_headless: Option<bool>,
    pub chrome_concurrent: Option<usize>,
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
            "Config {{\n    db_user: {}\n    db_password: {}\n    deno_server_url: {}\n    surrealdb_url: {}\n    chrome_headless: {:?}\n    chrome_concurrent: {:?}\n    chrome_data_dir: {:?}\n}}",
            "*".repeat(self.db_user.len()),
            "*".repeat(self.db_password.len()),
            self.deno_server_url,
            self.surrealdb_host,
            self.chrome_headless,
            self.chrome_concurrent,
            self.chrome_data_dir,
        )
    }
}
