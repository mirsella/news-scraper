use super::{GetNewsOpts, News};
use anyhow::{bail, Context, Result};
use headless_chrome::Tab;
use log::{error, info};
use std::{sync::Arc, thread, time::Duration};

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links: Vec<String> = tab
        .find_elements("a.stretched-link")
        .context("finding articles links")?
        .iter()
        .map(|el| {
            let href = el.get_attribute_value("href").unwrap().expect("no href ??");
            format!("https://rtbf.be{href}")
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
    tab.navigate_to("https://www.rtbf.be/en-continu")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    if let Ok(cookie) = tab.find_element("#didomi-notice-disagree-button") {
        cookie.click().context("clicking on cookie")?;
        thread::sleep(Duration::from_secs(1));
    }

    for _ in 0..10 {
        tab.wait_for_element("button.group")
            .context("wait_for_element load more")?
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
        let res = super::fetch_article(&url);
        let payload = match res {
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
                error!("fetch_article on {url}: {err:?}");
                continue;
            }
        };
        opts.tx.blocking_send(payload).unwrap();
    }
    Ok(())
}
