use super::{GetNewsOpts, News};
use anyhow::{bail, Context, Result};
use headless_chrome::Tab;
use log::{error, info};
use std::{sync::Arc, thread, time::Duration};

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links: Vec<String> = tab
        .find_elements(".views-row:not(.with-video) > a[href]")
        .context("find_elements .views-row:not(.with-video) > a[href]")?
        .iter()
        .map(|el| {
            let href = el.get_attribute_value("href").unwrap().expect("no href ??");
            format!("https://information.tv5monde.com{href}")
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version()?.user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://information.tv5monde.com/afrique")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    if let Ok(cookie) = tab.find_element("#didomi-notice-disagree-button") {
        cookie.click().context("clicking on cookie")?;
        thread::sleep(Duration::from_secs(1));
    }

    for _ in 0..3 {
        tab.wait_for_element(".pager_more")
            .context("wait_for_element .pager_more")?
            .click()?;
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
                error!("parse_article: {err}");
                bail!("parse_article: {err}");
            }
        };
        opts.tx.blocking_send(payload).unwrap();
    }
    Ok(())
}
