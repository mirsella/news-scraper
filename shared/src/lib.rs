use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt, future::Future, path::PathBuf};
use surrealdb::{engine::remote::ws::Client as DbClient, sql::Thing, Surreal};

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
    pub html_body: Cow<'static, str>,
    pub text_body: Cow<'static, str>,
    pub caption: Cow<'static, str>,
    pub date: surrealdb::sql::Datetime,
    pub link: Cow<'static, str>,
    pub note: Cow<'static, str>,
    pub provider: Cow<'static, str>,
    pub rating: Option<i64>,
    pub tags: Option<Vec<Cow<'static, str>>>,
    pub title: Cow<'static, str>,
    pub used: bool,
    pub locked: bool,
}

impl DbNews {
    pub async fn unlock(&mut self, db: Surreal<DbClient>) -> Result<&Self> {
        let id: Thing = self.id.clone().ok_or(anyhow!("no id"))?;
        match db
            .update::<Option<DbNews>>(("news", id))
            .merge(serde_json::json!({"locked": false }))
            .await
        {
            Ok(Some(new)) => {
                *self = new;
                Ok(self)
            }
            Err(e) => Err(anyhow!("{}", e)),
            _ => Err(anyhow!("no news found")),
        }
    }
    pub async fn lock(&mut self, db: Surreal<DbClient>) -> Result<&Self> {
        let id: Thing = self.id.clone().ok_or(anyhow!("no id"))?;
        match db
            .update::<Option<DbNews>>(("news", id))
            .merge(serde_json::json!({"locked": true }))
            .await
        {
            Ok(Some(new)) => {
                *self = new;
                Ok(self)
            }
            Err(e) => Err(anyhow!("{}", e)),
            _ => Err(anyhow!("no news found")),
        }
    }
    pub async fn get_nonrated(db: Surreal<DbClient>) -> Result<DbNews> {
        let news: Option<DbNews> = db
            .query(
                "select * from news where rating == none AND date > time::floor(time::now(), 1w) AND locked == false limit 1",
            )
            .await?
            .take(0)?;
        news.ok_or(anyhow!("no news found"))
    }
}

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

pub fn sanitize_html(html: &str) -> String {
    let tags = maplit::hashset![
        "b", "i", "u", "em", "strong", "strike", "code", "hr", "br", "div", "table", "thead",
        "caption", "tbody", "tr", "th", "td", "p", "a", "img", "h1", "h2", "h3", "h4", "h5", "h6",
        "section"
    ];
    let allowed_attributes = ["href", "title", "src", "alt", "colspan"];
    ammonia::Builder::new()
        .tags(tags)
        .link_rel(None)
        .add_generic_attributes(&allowed_attributes)
        .clean(html)
        .to_string()
}

pub fn extract_clean_text(html: &str) -> String {
    let s = nanohtml2text::html2text(html);
    let re = regex::Regex::new(r"\(?https?://[^\s]+").unwrap();
    let s = re.replace_all(&s, "").to_string();
    let s = s
        .split_whitespace()
        .map(|s| s.trim().replace('\n', ""))
        .collect::<Vec<String>>()
        .join(" ");
    s
}
