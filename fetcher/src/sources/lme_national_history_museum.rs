use super::GetNewsOpts;
use crate::sources::fetch_article;
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, info};
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
    if links.is_empty() {
        bail!("no links found");
    }
    for url in links {
        if opts.is_seen(&url) {
            continue;
        }
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
                debug!("fetch_article on {url}: {err}");
                continue;
            }
        };
        opts.tx.blocking_send(payload)?;
    }
    Ok(())
}
