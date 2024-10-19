automod::dir!("src/sources");

use anyhow::{anyhow, Context};
use chrono::{DateTime, Local};
use headless_chrome::Browser;
use log::{debug, trace};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use shared::News;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

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
    author: Value,
    favicon: String,
    content: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    published: DateTime<Local>,
    source: String,
    links: Vec<String>,
    ttr: f64,
}

fn deserialize_null_default<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_else(Local::now))
}

pub fn fetch_article(url: impl AsRef<str>) -> Result<ApiResponse, anyhow::Error> {
    let url = url.as_ref();
    debug!("fetching {}", url);
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
    let json_result: ApiResponse = response
        .into_json()
        .context("deserialize json response to ApiResponse struct")?;
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
    let json_result: ApiResponse = response
        .into_json()
        .context("deserialize json response to ApiResponse struct")?;
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
    ("fr::20minutes", fr_20minutes::get_news),
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
    ("lme::beetravel", lme_beetravel::get_news),
    ("lme::voyagespirates", lme_voyagespirates::get_news),
    ("lme::journaldesfemmes", lme_journaldesfemmes::get_news),
    ("lme::demotivateur", lme_demotivateur::get_news),
    ("lme::huffingpost", lme_huffingpost::get_news),
    ("africa::lemonde", africa_lemonde::get_news),
    ("africa::apanews", africa_apanews::get_news),
    ("africa::abidjan", africa_abidjan::get_news),
    ("africa::linfodrome", africa_linfodrome::get_news),
    ("africa::tv5monde", africa_tv5monde::get_news),
    ("africa::africanews", africa_africanews::get_news),
    ("be::rtbf", be_rtbf::get_news),
    ("be::rtl", be_rtl::get_news),
    ("be::lalibre", be_lalibre::get_news),
    ("quebec::lapresse", quebec_lapresse::get_news),
    ("quebec::journaldequebec", quebec_journaldequebec::get_news),
    ("quebec::montrealgazette", quebec_montrealgazette::get_news),
    ("quebec::qctonline", quebec_qctonline::get_news),
    ("quebec::thesuburban", quebec_thesuburban::get_news),
    ("quebec::24heures", quebec_24heures::get_news),
    ("quebec::tvanouvelles", quebec_tvanouvelles::get_news),
];
