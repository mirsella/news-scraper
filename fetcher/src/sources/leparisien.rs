use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace};
use std::sync::Arc;

const CATEGORIES: [&str; 7] = [
    "faits-divers",
    "politique",
    "economie",
    "societe",
    "sports",
    "culture-loisirs",
    "etudiant",
];

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = tab
        .find_elements("div[class^='story'] > a, *[class*='article__link']")
        .context("finding div > a")?
        .iter()
        .map(|a| {
            let mut link = a
                .get_attribute_value("href")
                .expect("getting href")
                .expect("no href on article");
            if !link.starts_with("http") {
                link.insert_str(0, "http:");
            }
            link
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.enable_stealth_mode()?;
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!("https://www.leparisien.fr/{category}"))
            .context("navigate_to")?;
        tab.wait_until_navigated()
            .context("category wait_until_navigated")?;
        // tab.activate().unwrap();
        if let Ok(cookies) = tab.find_element_by_xpath("//button[contains(text(), 'Accepter')]") {
            cookies.click().context("clicking on cookies")?;
            tab.wait_until_navigated()
                .context("cookies wait_until_navigated")?;
        }

        let links = get_articles_links(&tab)?;
        trace!("found {} links on {category}", links.len());
        for url in links {
            if opts.seen_urls.lock().unwrap().contains(&url) {
                trace!("already seen {url}");
                continue;
            }
            opts.seen_urls.lock().unwrap().push(url.clone());

            let mut res = super::fetch_article(&url);
            if let Err(err) = res {
                debug!("fetch_article: {}", err);
                if tab.navigate_to(&url).is_err() {
                    continue;
                }
                if tab.wait_until_navigated().is_err() {
                    continue;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
                let doc = tab.get_content()?;
                res = super::parse_article(&doc);
            }
            let payload = match res {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: "leparisien".to_string(),
                    date: res.published.parse().unwrap_or_else(|_| chrono::Utc::now()),
                    body: res.content,
                    link: url,
                }),
                Err(err) => {
                    debug!("parse_article: {}", err);
                    continue;
                }
            };
            if let Err(e) = opts.tx.blocking_send(payload) {
                error!("blocking_send: {e}");
                break;
            }
        }
    }
    Ok(())
}
