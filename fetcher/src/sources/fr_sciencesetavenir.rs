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
        .context("finding .alaune > div.visuel > a, a.overlay")?
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
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!("https://www.sciencesetavenir.fr/{category}/"))
            .context("navigate_to")?;
        tab.wait_for_elements(".alaune > div.visuel > a, a.overlay")
            .context("sciencesetavenir wait for element .alaune > div.visuel > a, a.overlay-une")?;

        let links = get_articles_links(&tab)
            .context("sciencesetavenir")
            .inspect_err(|_error| {
                let data = tab
                    .capture_screenshot(
                        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                        None,
                        None,
                        true,
                    )
                    .unwrap();
                std::fs::write("sciencesetavenir-error.png", data).unwrap();
            })?;
        trace!("found {} links on {category}", links.len());
        if links.is_empty() {
            return Err(anyhow::anyhow!("no links found"));
        }
        for url in links {
            if opts.seen_urls.read().unwrap().contains(&url) {
                trace!("already seen {url}");
                continue;
            }
            opts.seen_urls.write().unwrap().push(url.clone());

            let res = fetch_article(&url);
            let payload = match res {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: "sciencesetavenir".to_string(),
                    tags: vec![category.to_string(), "science".to_string()],
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
