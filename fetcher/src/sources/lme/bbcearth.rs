use super::GetNewsOpts;
use crate::sources::fetch_article;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, info, trace};
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    Ok(tab
        .find_elements("a.article-block:not(.video-card)")
        .context("finding a.article-block:not(.video-card)")?
        .iter()
        .filter_map(|a| {
            let mut link = a.get_attribute_value("href").unwrap().expect("a href");
            if link.starts_with("/quiz") {
                return None;
            }
            link.insert_str(0, "https://www.bbcearth.com");
            Some(link)
        })
        .collect())
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.bbcearth.com/nature")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;
    let links = get_articles_links(&tab).context("get_articles_links")?;
    info!("found {} articles", links.len());
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());
        let tags: Vec<_> = ["nature", "lemediaexperience"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let payload = match fetch_article(&url) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: "lme::bbcearth".to_string(),
                tags,
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
                body: res.content,
                link: url,
            }),
            Err(err) => {
                debug!("fetch_article on {url}: {err}");
                continue;
            }
        };
        opts.tx.blocking_send(payload)?;
    }
    Ok(())
}
