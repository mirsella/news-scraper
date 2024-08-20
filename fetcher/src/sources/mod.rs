automod::dir!("src/sources");

use anyhow::anyhow;
use headless_chrome::Browser;
use log::trace;
use serde::{Deserialize, Serialize};
use shared::News;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

/// (link, tags)
#[derive(Deserialize, Debug)]
pub struct SeenLink {
    pub link: String,
    pub tags: Vec<String>,
}

pub struct GetNewsOpts {
    pub browser: Browser,
    pub tx: Sender<anyhow::Result<News>>,
    pub seen_links: Arc<RwLock<Vec<SeenLink>>>,
    pub provider: String,
}
impl GetNewsOpts {
    // is the link seen with the current provider?
    pub fn is_seen(&self, link: &str) -> bool {
        let prefix_tag = extract_prefix_from_provider(&self.provider);
        if self
            .seen_links
            .read()
            .unwrap()
            .iter()
            .any(|seen_link| seen_link.link == link && seen_link.tags.contains(&prefix_tag))
        {
            trace!("already seen {} with provider {}", link, self.provider);
            return true;
        }
        false
    }
}

pub fn extract_prefix_from_provider(module: &str) -> String {
    module.split_once("::").unwrap().0.to_string()
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
pub static SOURCES: &[(&str, SourceFn)] = &[
    ("fr::google", fr_google::get_news),
    ("fr::lavoixdunord", fr_lavoixdunord::get_news),
    ("fr::goodnewsnetwork", fr_goodnewsnetwork::get_news),
    ("fr::positivr", fr_positivr::get_news),
    ("fr::reporterre", fr_reporterre::get_news),
    ("fr::sciencesetavenir", fr_sciencesetavenir::get_news),
    (
        "fr::reddit-upliftingnews",
        fr_reddit_upliftingnews::get_news,
    ),
    ("fr::20minutes", fr_twentyminutes::get_news),
    ("fr::sudouest", fr_sudouest::get_news),
    ("fr::ouest-france", fr_ouest_france::get_news),
    ("fr::leparisien", fr_leparisien::get_news),
    ("fr::francetvinfo", fr_francetvinfo::get_news),
    ("fr::futura-sciences", fr_futura_sciences::get_news),
    (
        "lme::national-history-museum",
        lme_national_history_museum::get_news,
    ),
    // ("lme::travelandleisure", lme_travelandleisure::get_news),
    // ("lme::bbc", lme_bbc::get_news),
    ("lme::capturetheatlas", lme_capturetheatlas::get_news),
    ("lme::bbcearth", lme_bbcearth::get_news),
    ("lme::smithsonianmag", lme_smithsonianmag::get_news),
    ("lme::geo", lme_geo::get_news),
    ("lme::nationalgeographic", lme_nationalgeographic::get_news),
    ("lme::theguardian", lme_theguardian::get_news),
    ("lme::futura-sciences", lme_futura_sciences::get_news),
    ("africa::lemonde", africa_lemonde::get_news),
    ("africa::apanews", africa_apanews::get_news),
    ("africa::abidjan", africa_abidjan::get_news),
    ("africa::linfodrome", africa_linfodrome::get_news),
    ("africa::tv5monde", africa_tvfivemonde::get_news),
    ("africa::africanews", africa_africanews::get_news),
    // belgium
    ("be::rtbf", be_rtbf::get_news),
    ("be::rtl", be_rtl::get_news),
    ("be::lalibre", be_lalibre::get_news),
    // quebec
];
