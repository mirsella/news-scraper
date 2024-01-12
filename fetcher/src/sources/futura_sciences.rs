use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, trace};
use shared::News;
use std::{str::pattern::Pattern, sync::Arc};

use crate::sources::fetch_article;

use super::GetNewsOpts;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = tab
        .find_elements(".article-card-box")
        .context("finding .article-card-box")?
        .iter()
        .map(|a| {
            let mut link = a
                .get_attribute_value("href")
                .expect("getting href")
                .expect("no href on article");
            link.insert_str(0, "https://futura-sciences.com");
            link
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.futura-sciences.com/sitemap-html/actualites/")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;
    let links = get_articles_links(&tab)?;
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());

        let tags = url
            .strip_prefix("https://www.futura-sciences.com/")
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
                provider: "futura-sciences".to_string(),
                tags,
                date: res.published.parse().unwrap_or_else(|_| chrono::Utc::now()),
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
