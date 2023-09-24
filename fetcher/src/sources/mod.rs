use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use shared::News;

use tokio::sync::mpsc::Sender;
automod::dir!("src/sources");

pub struct GetNewsOpts {
    pub browser: headless_chrome::Browser,
    pub tx: Sender<anyhow::Result<News>>,
    pub seen_urls: Arc<Mutex<Vec<String>>>,
}
type GetNewsFn = fn(GetNewsOpts) -> anyhow::Result<()>;

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

pub fn fetch_article(url: &str) -> Result<ApiResponse, anyhow::Error> {
    let endpoint = format!(
        "{}/fetch?url={}",
        std::env::var("deno_server_url").expect("DENO_SERVER_URL not set"),
        url
    );
    let response = ureq::get(&endpoint)
        .timeout(Duration::from_secs(5))
        .call()?;

    if response.status() != 200 {
        return Err(anyhow::anyhow!(
            "deno server returned status code {}. err: {}",
            response.status(),
            response.into_string()?
        ));
    }
    let json_result: ApiResponse = response.into_json()?;
    Ok(json_result)
}
pub fn parse_article(str: &str) -> Result<ApiResponse, anyhow::Error> {
    let endpoint = format!(
        "{}/parse",
        std::env::var("deno_server_url").expect("DENO_SERVER_URL not set")
    );
    let response = ureq::post(&endpoint)
        .timeout(Duration::from_secs(5))
        .send_string(str)?;

    if response.status() != 200 {
        return Err(anyhow::anyhow!(
            "deno server returned status code {}. err: {}",
            response.status(),
            response.into_string()?
        ));
    }
    let json_result: ApiResponse = response.into_json()?;
    Ok(json_result)
}

pub static SOURCES: [(&str, GetNewsFn); 4] = [
    ("francetvinfo", francetvinfo::get_news),
    ("google", google::get_news),
    ("leparisien", leparisien::get_news),
    ("reporterre", reporterre::get_news),
];
