mod lme;
mod news;

use anyhow::anyhow;
use headless_chrome::Browser;
use serde::{Deserialize, Serialize};
use shared::News;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

pub struct GetNewsOpts {
    pub browser: Browser,
    pub tx: Sender<anyhow::Result<News>>,
    pub seen_urls: Arc<RwLock<Vec<String>>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiResponse {
    url: String,
    title: String,
    description: String,
    image: String,
    author: String,
    favicon: String,
    content: String,
    published: String,
    source: String,
    links: Vec<String>,
    ttr: f64,
}

pub fn fetch_article(url: impl AsRef<str>) -> Result<ApiResponse, anyhow::Error> {
    let url = url.as_ref();
    let endpoint = format!(
        "{}/fetch?url={}",
        std::env::var("ARTICLE_PARSER_URL").expect("ARTICLE_PARSER_URL not set"),
        url
    );
    let response = ureq::get(&endpoint).timeout(Duration::from_secs(6)).call();
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(code, res)) => {
            return Err(anyhow!("{code}: {:#?}", res));
        }
        Err(e) => {
            return Err(anyhow!("{url}: {e}"));
        }
    };
    let json_result: ApiResponse = response.into_json()?;
    Ok(json_result)
}
pub fn parse_article(str: impl AsRef<str>) -> Result<ApiResponse, anyhow::Error> {
    let str = str.as_ref();
    let endpoint = format!(
        "{}/parse",
        std::env::var("ARTICLE_PARSER_URL").expect("ARTICLE_PARSER_URL not set")
    );
    let response = ureq::post(&endpoint)
        .timeout(Duration::from_secs(5))
        .send_string(str);
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(code, res)) => {
            return Err(anyhow!("{}: {:#?}", code, res));
        }
        Err(e) => {
            return Err(anyhow!("{}", e));
        }
    };
    let json_result: ApiResponse = response.into_json()?;
    Ok(json_result)
}

pub type SourceFn = fn(GetNewsOpts) -> anyhow::Result<()>;
pub type SourceType = &'static [(&'static str, SourceFn)];
pub static SOURCES: [(&str, SourceType); 2] = [("news", &news::SOURCES), ("lme", &lme::SOURCES)];
