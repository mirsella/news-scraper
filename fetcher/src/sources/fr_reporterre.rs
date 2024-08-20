use super::{GetNewsOpts, News};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, trace};
use std::sync::Arc;

const CATEGORIES: [&str; 8] = [
    "Nature",
    "Climat-18",
    "Luttes",
    "Alternatives",
    "International",
    "Reportage",
    "Enquete",
    "idee",
];

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = tab
        .find_elements(".lien_article")
        .context("finding .lien_article")?
        .iter()
        .map(|a| {
            let mut link = a
                .get_attribute_value("href")
                .expect("getting href")
                .expect("no href on article");
            link.insert_str(0, "https://reporterre.net/");
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
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!("https://reporterre.net/{category}"))
            .context("navigate_to")?;
        tab.wait_until_navigated()
            .context("category wait_until_navigated")?;

        let links = get_articles_links(&tab).context("reporterre")?;
        trace!("found {} links on {category}", links.len());
        if links.is_empty() {
            bail!("no links found");
        }
        for url in links {
            if opts.is_seen(&url) {
                continue;
            }

            let payload = match super::fetch_article(&url) {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: opts.provider.clone(),
                    tags: vec![category.to_string()],
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
    }
    Ok(())
}
