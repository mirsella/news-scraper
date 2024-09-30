use super::{GetNewsOpts, News};
use anyhow::{bail, Context, Result};
use headless_chrome::Tab;
use log::{error, info};
use std::{collections::HashSet, sync::Arc, thread, time::Duration};

fn get_articles_links(tab: &Arc<Tab>) -> Result<HashSet<String>> {
    let links = tab
        .find_elements(".news_unit-link")
        .context("find_elements articles links")?
        .iter()
        .map(|el| el.get_attribute_value("href").unwrap().expect("no href ??"))
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version()?.user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.tvanouvelles.ca/regional")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    if let Ok(cookie) = tab.find_element(".fc-button.fc-cta-consent") {
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
        tab.navigate_to(&url)
            .context("navigate_to article")?
            .wait_until_navigated()
            .context("wait_until_navigated article")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        let doc = tab.get_content().context("get_content")?;
        let res = super::parse_article(&doc);
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
                continue;
            }
        };
        opts.tx.blocking_send(payload).unwrap();
    }
    Ok(())
}
