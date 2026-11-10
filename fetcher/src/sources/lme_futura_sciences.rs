use super::{absolute_url, dedupe_links, GetNewsOpts};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, info};
use shared::News;
use std::sync::Arc;

const CATEGORIES: [&str; 3] = ["planete/voyage", "planete/environnement", "planete/terre"];

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = dedupe_links(
        tab.find_elements("a.article-link.article-title")
            .context("finding article links")?
            .iter()
            .filter_map(|a| {
                let link = a.get_attribute_value("href").unwrap()?;
                let slug = link.trim_end_matches('/').rsplit('/').next()?;
                if link.contains("/personnalites")
                    || link.contains("/live")
                    || !slug.ends_with(|c: char| c.is_ascii_digit())
                {
                    return None;
                }
                Some(absolute_url("https://futura-sciences.com", &link))
            }),
    );
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for category in CATEGORIES {
        debug!("checking out category {category}");
        tab.navigate_to(&format!("https://www.futura-sciences.com/{category}/"))
            .context("navigate_to")?
            .wait_until_navigated()
            .context("wait_until_navigated")?;
        let links = get_articles_links(&tab).context(opts.provider.clone())?;
        info!("found {} articles in category {category}", links.len());
        if links.is_empty() {
            bail!("no links found");
        }
        for url in links {
            if opts.is_seen(&url) {
                continue;
            }
            let payload = match super::fetch_article_with_browser_fallback(&opts.browser, &url) {
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
                    log::warn!("fetch_article on {url}: {err:?}");
                    continue;
                }
            };
            opts.tx.blocking_send(payload)?;
        }
    }
    Ok(())
}
