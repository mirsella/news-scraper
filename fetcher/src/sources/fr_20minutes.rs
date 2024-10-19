use super::{GetNewsOpts, News};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{error, trace};
use std::{sync::Arc, thread, time::Duration};

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    for _ in 0..5 {
        tab.find_element("#latest-articles-container > div > button")
            .context("find_element `Voir plus d’articles`")?
            .click()
            .context("click `Voir plus d’articles`")?;
        thread::sleep(Duration::from_secs(5));
    }
    let links: Vec<String> = tab
        .find_elements("#latest-articles-container a")
        .expect("latest-articles-container articles not found")
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
    tab.navigate_to("https://www.20minutes.fr/actus")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    if let Ok(cookie) = tab.find_element("#didomi-notice-agree-button") {
        cookie.click().context("clicking on cookie")?;
        std::thread::sleep(std::time::Duration::from_secs(3));
        trace!("clicked cookie");
    }

    let links = get_articles_links(&tab)?;
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
date: res.published,
                body: res.content,
                link: url,
                ..Default::default()
            }),
            Err(err) => {
                log::warn!("fetch_article on {url}: {err}");
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
