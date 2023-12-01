use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace};
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links: Vec<String> = tab
        .find_elements(".r-direct--link")
        .expect(".r-direct--link")
        .iter()
        .map(|el| {
            let mut link = el
                .get_attribute_value("href")
                .unwrap()
                .expect("lavoixdunord no href ??");
            link.insert_str(0, "https://www.lavoixdunord.fr");
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
    tab.navigate_to("https://www.lavoixdunord.fr/fil-info")
        .context("navigate_to")?;

    let links = get_articles_links(&tab).context("lavoixdunord get_articles_links")?;
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());
        let payload = match super::fetch_article(&url) {
            Ok(res) => Ok(News {
                tags: vec!["france".to_string()],
                title: res.title,
                caption: res.description,
                provider: "lavoixdunord".to_string(),
                date: res.published.parse().unwrap_or_else(|_| chrono::Utc::now()),
                body: res.content,
                link: url,
            }),
            Err(err) => {
                debug!("fetch_article: {err}");
                continue;
            }
        };
        if let Err(e) = opts.tx.blocking_send(payload) {
            error!("blocking_send: {e}");
            break;
        }
    }
    Ok(())
}
