use super::{fetch_article, GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, trace};
use std::sync::Arc;

const CATEGORIES: [&str; 5] = ["sciences", "sante", "tech", "maison", "planete"];

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
    let tab = opts.browser.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!(
            "https://futura-sciences.com/{category}/actualites"
        ))
        .context("navigate_to")?;
        tab.wait_until_navigated()
            .context("category wait_until_navigated")?;

        let links = get_articles_links(&tab).context("futura-sciences")?;
        trace!("found {} links on {category}", links.len());
        for url in links {
            if opts.seen_urls.lock().unwrap().contains(&url) {
                trace!("already seen {url}");
                continue;
            }
            opts.seen_urls.lock().unwrap().push(url.clone());

            let res = fetch_article(&url);
            let payload = match res {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: "futura-sciences".to_string(),
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
    }
    Ok(())
}
