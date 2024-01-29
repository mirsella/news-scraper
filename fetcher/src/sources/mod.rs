mod lme;

use anyhow::anyhow;
use headless_chrome::Browser;
use serde::{Deserialize, Serialize};
use shared::News;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

automod::dir!("src/sources");

pub struct GetNewsOpts {
    pub browser: Browser,
    pub config: Arc<shared::Config>,
    pub tx: Sender<anyhow::Result<News>>,
    pub seen_urls: Arc<Mutex<Vec<String>>>,
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

type SourceFn = fn(GetNewsOpts) -> anyhow::Result<()>;
pub static SOURCES: [(&str, SourceFn); 22] = [
    ("francetvinfo", francetvinfo::get_news),
    ("google", google::get_news),
    ("leparisien", leparisien::get_news),
    ("reporterre", reporterre::get_news),
    ("futura-sciences", futura_sciences::get_news),
    ("sciencesetavenir", sciencesetavenir::get_news),
    ("reddit-upliftingnews", reddit_upliftingnews::get_news),
    ("goodnewsnetwork", goodnewsnetwork::get_news),
    ("positivr", positivr::get_news),
    ("ouest-france", ouest_france::get_news),
    ("20minutes", twentyminutes::get_news),
    ("sudouest", sudouest::get_news),
    ("lavoixdunord", lavoixdunord::get_news),
    ("lme::futura-sciences", lme::futura_sciences::get_news),
    ("lme::geo", lme::geo::get_news),
    ("lme::nationalgeographic", lme::nationalgeographic::get_news),
    ("lme::capturetheatlas", lme::capturetheatlas::get_news),
    // ("lme::travelandleisure", lme::travelandleisure::get_news),
    ("lme::bbcearth", lme::bbcearth::get_news),
    ("lme::bbc", lme::bbc::get_news),
    ("lme::theguardian", lme::theguardian::get_news),
    ("lme::smithsonianmag", lme::smithsonianmag::get_news),
    (
        "lme::national-history-museum",
        lme::national_history_museum::get_news,
    ),
];
