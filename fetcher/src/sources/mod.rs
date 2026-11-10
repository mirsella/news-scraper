automod::dir!("src/sources");

use anyhow::Context;
use chrono::{DateTime, Local};
use headless_chrome::{Browser, Tab};
use log::{debug, trace};
use serde::{Deserialize, Deserializer, Serialize};
use shared::News;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

pub struct GetNewsOpts {
    pub browser: Browser,
    pub tx: Sender<anyhow::Result<News>>,
    pub seen_links: Arc<RwLock<Vec<String>>>,
    pub provider: String,
}
impl GetNewsOpts {
    // is the link seen with the current provider?
    pub fn is_seen(&self, link: &str) -> bool {
        if self.seen_links.read().unwrap().contains(&link.to_string()) {
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
    #[serde(deserialize_with = "deserialize_null_default")]
    published: DateTime<Local>,
    source: String,
    links: Vec<String>,
    ttr: f64,
    r#type: String,
}

fn deserialize_null_default<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(deserializer).unwrap_or_default();
    let parsed = dateparser::parse(&string);
    Ok(parsed.map(Into::into).unwrap_or_else(|_| Local::now()))
}

pub fn absolute_url(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if let Some(rest) = href.strip_prefix("//") {
        format!("https://{rest}")
    } else {
        let origin = base.split('/').take(3).collect::<Vec<_>>().join("/");
        if href.starts_with('/') {
            format!("{origin}{href}")
        } else {
            format!("{origin}/{href}")
        }
    }
}

pub fn dedupe_links(links: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    links
        .into_iter()
        .filter(|link| seen.insert(link.clone()))
        .collect()
}

fn has_article_element(tab: &Arc<Tab>) -> bool {
    tab.evaluate("!!document.querySelector('article')", false)
        .ok()
        .and_then(|result| result.value)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub fn parse_url_with_browser(tab: &Arc<Tab>, url: &str) -> Result<ApiResponse, anyhow::Error> {
    tab.navigate_to(url)
        .with_context(|| format!("navigate_to {url}"))?
        .wait_until_navigated()
        .with_context(|| format!("wait_until_navigated {url}"))?;
    for _ in 0..10 {
        if has_article_element(tab) {
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    std::thread::sleep(Duration::from_millis(300));
    let document = tab.get_content().context("get_content")?;
    parse_article(document)
}

pub fn fetch_article_with_browser_fallback(
    browser: &Browser,
    url: &str,
) -> Result<ApiResponse, anyhow::Error> {
    match fetch_article(url) {
        Ok(article) => Ok(article),
        Err(error) => {
            debug!("fetch failed for {url}: {error:#}; retrying in browser");
            let tab = browser.new_tab().context("opening fallback tab")?;
            if let Ok(version) = browser.get_version() {
                let user_agent = version.user_agent.replace("HeadlessChrome", "Chrome");
                let _ = tab.set_user_agent(&user_agent, None, None);
            }
            tab.set_default_timeout(Duration::from_secs(30));
            let result = parse_url_with_browser(&tab, url);
            let _ = tab.close(false);
            result
        }
    }
}

pub fn fetch_article(url: impl AsRef<str>) -> Result<ApiResponse, anyhow::Error> {
    let url = url.as_ref();
    debug!("fetching {}", url);
    let endpoint = format!(
        "{}/fetch?url={}",
        std::env::var("ARTICLE_PARSER_URL").expect("ARTICLE_PARSER_URL not set"),
        url
    );
    let mut response = ureq::get(&endpoint)
        .config()
        .timeout_global(Some(Duration::from_secs(6)))
        .build()
        .call()
        .with_context(|| format!("fetching article {url}"))?;
    let article: ApiResponse = response
        .body_mut()
        .read_json()
        .context("deserialize json response to ApiResponse struct")?;
    if shared::extract_clean_text(&article.content).len() < 50 {
        anyhow::bail!("parser returned no article text for {url}");
    }
    Ok(article)
}

pub fn parse_article(str: impl AsRef<str>) -> Result<ApiResponse, anyhow::Error> {
    let str = str.as_ref();
    let endpoint = format!(
        "{}/parse",
        std::env::var("ARTICLE_PARSER_URL").expect("ARTICLE_PARSER_URL not set")
    );
    let mut response = ureq::post(&endpoint)
        .config()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .header("Content-Type", "text/plain; charset=utf-8")
        .send(str)
        .context("requesting article parsing")?;
    let article: ApiResponse = response
        .body_mut()
        .read_json()
        .context("deserialize json response to ApiResponse struct")?;
    if shared::extract_clean_text(&article.content).len() < 50 {
        anyhow::bail!("parser returned no article text");
    }
    Ok(article)
}

pub type SourceFn = fn(GetNewsOpts) -> anyhow::Result<()>;
pub static SOURCES: &[(&str, SourceFn)] = &[
    // Google search and its RSS links hit a consent/CAPTCHA wall from this host.
    // ("fr::google", fr_google::get_news),
    ("fr::bing", fr_bing::get_news),
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
    // Cloudflare blocks headless article requests.
    // ("lme::smithsonianmag", lme_smithsonianmag::get_news),
    ("lme::geo", lme_geo::get_news),
    ("lme::nationalgeographic", lme_nationalgeographic::get_news),
    ("lme::theguardian", lme_theguardian::get_news),
    ("lme::futura-sciences", lme_futura_sciences::get_news),
    ("lme::beetravel", lme_beetravel::get_news),
    ("lme::voyagespirates", lme_voyagespirates::get_news),
    ("lme::journaldesfemmes", lme_journaldesfemmes::get_news),
    ("lme::demotivateur", lme_demotivateur::get_news),
    ("lme::huffingtonpost", lme_huffingtonpost::get_news),
    // Cloudflare / paywall blocks headless article requests.
    // ("africa::lemonde", africa_lemonde::get_news),
    // ("africa::apanews", africa_apanews::get_news),
    ("africa::abidjan", africa_abidjan::get_news),
    ("africa::linfodrome", africa_linfodrome::get_news),
    ("africa::tv5monde", africa_tv5monde::get_news),
    ("africa::africanews", africa_africanews::get_news),
    ("be::rtbf", be_rtbf::get_news),
    ("be::rtl", be_rtl::get_news),
    ("be::lalibre", be_lalibre::get_news),
    ("quebec::lapresse", quebec_lapresse::get_news),
    // Quebecor sites return Access Denied for article pages from data centres.
    // ("quebec::journaldequebec", quebec_journaldequebec::get_news),
    ("quebec::montrealgazette", quebec_montrealgazette::get_news),
    ("quebec::qctonline", quebec_qctonline::get_news),
    ("quebec::thesuburban", quebec_thesuburban::get_news),
    // ("quebec::24heures", quebec_24heures::get_news),
    // ("quebec::tvanouvelles", quebec_tvanouvelles::get_news),
];
