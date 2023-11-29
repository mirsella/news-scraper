use std::{
    ffi::OsStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::anyhow;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use serde::{Deserialize, Serialize};
use shared::News;

use tokio::sync::mpsc::Sender;
automod::dir!("src/sources");

#[derive(Debug, Clone)]
pub struct GetNewsOpts {
    // pub browser: headless_chrome::Browser,
    pub config: shared::Config,
    pub tx: Sender<anyhow::Result<News>>,
    pub seen_urls: Arc<Mutex<Vec<String>>>,
}
impl GetNewsOpts {
    fn new_browser(&self, image_enabled: bool) -> headless_chrome::Browser {
        let mut args = Vec::new();
        if !image_enabled {
            args.push(OsStr::new("--blink-settings=imagesEnabled=false"));
        }
        Browser::new(
            LaunchOptionsBuilder::default()
                .window_size(Some((1920, 1080)))
                .headless(self.config.chrome_headless.unwrap_or(true))
                .user_data_dir(self.config.chrome_data_dir.clone())
                .args(args)
                .idle_browser_timeout(Duration::from_secs(60))
                .sandbox(false)
                .build()
                .unwrap(),
        )
        .unwrap()
    }
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

pub fn fetch_article(url: &str) -> Result<ApiResponse, anyhow::Error> {
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
pub fn parse_article(str: &str) -> Result<ApiResponse, anyhow::Error> {
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
pub static SOURCES: [(&str, SourceFn); 13] = [
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
];
