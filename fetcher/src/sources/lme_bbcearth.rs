use super::GetNewsOpts;
use crate::sources::fetch_article;
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::info;
use shared::News;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    Ok(tab
        .find_elements(".article-block")
        .context("finding articles links")?
        .iter()
        .filter_map(|a| {
            let mut link = a.get_attribute_value("href").unwrap().expect("a href");
            if link.contains("/quiz/") {
                return None;
            }
            link.insert_str(0, "https://www.bbcearth.com");
            Some(link)
        })
        .collect())
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.bbcearth.com")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;
    thread::sleep(Duration::from_secs(3));
    let links = get_articles_links(&tab).context("get_articles_links")?;
    info!("found {} articles", links.len());
    if links.is_empty() {
        bail!("no links found");
    }
    for url in links {
        if opts.is_seen(&url) {
            continue;
        }
        let tags: Vec<_> = ["nature", "lemediaexperience"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let payload = match fetch_article(&url) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: opts.provider.clone(),
                tags,
date: res.published,
                body: res.content,
                link: url,
            }),
            Err(err) => {
                log::warn!("fetch_article on {url}: {err}");
                continue;
            }
        };
        opts.tx.blocking_send(payload)?;
    }
    Ok(())
}
