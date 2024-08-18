use super::GetNewsOpts;
use crate::sources::fetch_article;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, info, trace};
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    Ok(tab
        .find_elements("a[class^='ImageGrid_ImageGrid__container__']")
        .context("find_elements links")?
        .iter()
        .map(|el| {
            "https://www.nhm.ac.uk".to_string()
                + &el.get_attribute_value("href").unwrap().expect("a href")
        })
        .collect())
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.nhm.ac.uk/wpy/peoples-choice")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;
    let links = get_articles_links(&tab).context("get_articles_links")?;
    info!("found {} articles", links.len());
    for url in links {
        if opts.seen_urls.read().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.write().unwrap().push(url.clone());
        let tags: Vec<_> = [
            "national history museum",
            "wildlife",
            "photography",
            "nhm",
            "wpo",
            "lemediaexperience",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let payload = match fetch_article(&url) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: "lme::national-history-museum".to_string(),
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
