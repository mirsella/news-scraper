use super::GetNewsOpts;
use crate::sources::fetch_article;
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, info};
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = tab
        .find_elements(".article-card-box, .keen-slider__slide")
        .context("finding .article-card-box")?
        .iter()
        .filter_map(|a| {
            if let Some(mut link) = a.get_attribute_value("href").unwrap() {
                if link.contains("personnalites") {
                    return None;
                }
                link.insert_str(0, "https://futura-sciences.com");
                return Some(link);
            }
            None
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.futura-sciences.com/sitemap-html/actualites/")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;

    let links = get_articles_links(&tab).context("futura-sciences")?;
    info!("found {} articles", links.len());
    if links.is_empty() {
        bail!("no links found");
    }
    for url in links {
        if opts.is_seen(&url) {
            continue;
        }

        let tags = url
            .strip_prefix("https://futura-sciences.com/")
            .expect(&url)
            .split('/')
            .take(2)
            .map(str::to_string)
            .collect();

        let res = fetch_article(&url);
        let payload = match res {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: opts.provider.clone(),
                tags,
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
                body: res.content,
                link: url,
            }),
            Err(err) => {
                debug!("fetch_article: {}", err);
                continue;
            }
        };
        opts.tx.blocking_send(payload)?;
    }
    Ok(())
}
