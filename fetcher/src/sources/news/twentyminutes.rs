use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace};
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    tab.find_element(".infinite-more-btn")?.scroll_into_view()?;
    let links: Vec<String> = tab
        .find_elements("article > a")
        .expect("article not found")
        .iter()
        .map(|el| {
            let mut link = el.get_attribute_value("href").unwrap().expect("no href ??");
            link.insert_str(0, "https://www.20minutes.fr");
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
    tab.navigate_to("https://www.20minutes.fr/actus")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    let links = get_articles_links(&tab).context("20minutes")?;
    assert!(!links.is_empty());
    for url in links {
        if opts.seen_urls.read().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.write().unwrap().push(url.clone());
        let payload = match super::fetch_article(&url) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: "20minutes".to_string(),
                tags: vec!["france".to_string()],
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
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
