use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace};
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links: Vec<String> = tab
        .find_elements(".article-wrapper > a")
        .expect(".article-wrapper > a")
        .iter()
        .filter_map(|el| {
            let mut link = el.get_attribute_value("href").unwrap().expect("no href ??");
            if !link.starts_with("http") {
                link.insert_str(0, "https://www.sudouest.fr");
            }
            if link.contains("youtube.com") {
                return None;
            }
            Some(link)
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.sudouest.fr/")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;

    if let Ok(cookie) = tab.find_element("#didomi-notice-agree-button") {
        trace!("clicking cookie");
        cookie.click().context("clicking on cookie")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    let links = get_articles_links(&tab).context("sudouest")?;
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());

        let payload = match super::fetch_article(&url) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: "sudouest".to_string(),
                tags: vec!["france".to_string()],
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
