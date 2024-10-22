use super::GetNewsOpts;
use crate::sources::parse_article;
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::info;
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    Ok(tab
        .find_elements("h2.articlePreview-title > a")
        .context("finding articles links")?
        .iter()
        .filter_map(|a| {
            let link = a.get_attribute_value("href").unwrap().expect("a href");
            if link.ends_with("geo.fr/") {
                return None;
            }
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
    for category in ["evenement/vacances-en-france", "voyage", "aventure"] {
        tab.navigate_to(&format!("https://www.geo.fr/{category}"))
            .context("navigate_to")?
            .wait_until_navigated()
            .context("wait_until_navigated")?;
        let links = get_articles_links(&tab).context("get_articles_links")?;
        info!("found {} articles", links.len());
        if links.is_empty() {
            bail!("no links found");
        }
        for url in links {
            if opts.is_seen(&url) {
                continue;
            }
            tab.navigate_to(&url)
                .context(format!("navigate_to {url}"))?
                .wait_until_navigated()
                .context("wait_until_navigated url")?;
            let body = tab.get_content()?;
            let payload = match parse_article(&body) {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: opts.provider.clone(),
                    date: res.published,
                    body: res.content,
                    link: url,
                    ..Default::default()
                }),
                Err(err) => {
                    log::warn!("parse_article on {url}: {err:?}");
                    continue;
                }
            };
            opts.tx.blocking_send(payload)?;
        }
    }
    Ok(())
}
