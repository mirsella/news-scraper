use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace, warn};
use std::sync::Arc;

const KEYWORDS: [&str; 4] = ["bonne nouvelle", "joie", "optimisme", "entraide"];

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let articles = tab
        .find_elements_by_xpath(
            "/html/body/div[6]/div/div[11]/div/div[2]/div[2]/div/div/div/div/*/div/div/a",
        )
        .context("finding parent of articles")?;
    let links = articles
        .iter()
        .map(|a| {
            a.get_attribute_value("href")
                .expect("getting href")
                .expect("no href on article")
        })
        .collect();
    Ok(links)
}

pub fn get_news(mut opts: GetNewsOpts) -> Result<()> {
    let (tab, tx) = (opts.tab, opts.tx);
    for keyword in KEYWORDS {
        trace!("checking out keyword {keyword}");
        tab.navigate_to(&format!(
            "https://www.google.com/search?q={}&tbm=nws&tbs=qdr%3Ad",
            keyword
        ))
        .context("navigate_to")?;
        tab.wait_until_navigated().context("wait_until_navigated")?;
        if let Ok(cookies) = tab.find_element_by_xpath("//span[contains(text(), 'Tout refuser')]") {
            cookies.click().context("clicking on cookies")?;
            tab.wait_until_navigated()?;
        }
        tab.wait_for_element("#center_col")
            .context("waiting on #center_col")?;

        let links = get_articles_links(&tab)?;
        trace!("found {} links on {keyword}", links.len());
        for url in links {
            if opts.seen_urls.contains(&url) {
                trace!("already seen {url}");
                continue;
            }
            opts.seen_urls.push(url.clone());

            let mut res = super::fetch_article(&url);
            if let Err(err) = res {
                trace!("fetch_article: {:#?}", err);
                tab.navigate_to(&url)?;
                tab.wait_until_navigated().context("wait_until_navigated")?;
                if let Ok(el) = tab.find_element_by_xpath("//*[contains(text(), 'Accepter')]") {
                    el.click()?;
                    tab.wait_until_navigated()?;
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
                let doc = tab.get_content()?;
                res = super::parse_article(&doc);
            }
            let payload = match res {
                Ok(res) => Ok(News {
                    title: res.title,
                    description: res.description,
                    provider: "google".to_string(),
                    time: res.published.parse().unwrap_or_else(|_| chrono::Utc::now()),
                    body: res.content,
                    link: res.url,
                }),
                Err(err) => {
                    debug!("parse_article: {:#?}", err);
                    // Err(err)
                    continue;
                }
            };
            let tx = tx.clone();
            if let Err(e) = tx.blocking_send(payload) {
                error!("blocking_send: {e:?}");
                break;
            }
        }
    }
    Ok(())
}
