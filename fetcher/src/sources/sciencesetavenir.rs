use super::{fetch_article, GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, trace};
use std::sync::Arc;

const CATEGORIES: [&str; 8] = [
    "espace",
    "sante",
    "nutrition",
    "nature-environnement",
    "animaux",
    "high-tech",
    "archeo-paleo",
    "fondamental",
];

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = tab
        .find_elements(".alaune > div.visuel > a, a.overlay")
        .context("finding a[href^='https://www.sciencesetavenir.fr/{category}']")?
        .iter()
        .map(|a| {
            a.get_attribute_value("href")
                .expect("getting href")
                .expect("no href on article")
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.enable_stealth_mode()?;
    // let user_agent = opts.browser.get_version().unwrap().user_agent;
    // let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    // tab.set_user_agent(&user_agent, None, None)?;
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!("https://www.sciencesetavenir.fr/{category}/"))
            .context("navigate_to")?;
        tab.wait_for_element(".content-une")
            .context("sciencesetavenir wait for element .content-une")?;

        let links = get_articles_links(&tab).context("sciencesetavenir")?;
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
                    provider: "sciencesetavenir".to_string(),
                    tags: vec!["france".to_string(), "science".to_string()],
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
