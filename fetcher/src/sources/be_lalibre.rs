use super::{GetNewsOpts, News};
use anyhow::{bail, Context, Result};
use headless_chrome::Tab;
use log::{error, info};
use std::{sync::Arc, thread, time::Duration};

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links: Vec<String> = tab
        .find_elements(".ap-StoryList-itemLink:not([href^='/dossier']):not([href^='/videos'])")
        .context("find_elements articles links")?
        .iter()
        .map(|el| {
            let href = el.get_attribute_value("href").unwrap().expect("no href ??");
            tab.get_url() + &href
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version()?.user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.lalibre.be/belgique")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    if let Ok(cookie) = tab.wait_for_element("#didomi-notice-agree-button") {
        cookie.click().context("clicking on cookie")?;
        thread::sleep(Duration::from_secs(1));
    }

    let links = get_articles_links(&tab)?;
    info!("found {} articles", links.len());
    if links.is_empty() {
        bail!("no links found");
    }

    for url in links {
        if opts.is_seen(&url) {
            continue;
        }
        let res = super::fetch_article(&url);
        let payload = match res {
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
                ..Default::default()
            }),
            Err(err) => {
                error!("fetch_article: {err}");
                bail!("fetch_article: {err}");
            }
        };
        opts.tx.blocking_send(payload).unwrap();
    }
    Ok(())
}
