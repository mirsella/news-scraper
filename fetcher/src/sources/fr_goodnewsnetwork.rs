use super::{GetNewsOpts, News};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::error;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links: Vec<String> = tab
        .find_elements(".entry-title > a")
        .expect(".entry-title > a")
        .iter()
        .map(|el| el.get_attribute_value("href").unwrap().expect("no href ??"))
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.goodnewsnetwork.org/category/news")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    let links = get_articles_links(&tab).context("goodnewsnetwork")?;
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
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
                body: res.content,
                link: url,
                tags: vec!["usa/world".to_string(), "goodnews".to_string()],
            }),
            Err(err) => {
                log::warn!("fetch_article: {err}");
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
