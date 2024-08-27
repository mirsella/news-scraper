use super::{GetNewsOpts, News};
use anyhow::{bail, Context, Result};
use headless_chrome::Tab;
use log::{debug, error, info, trace};
use std::{sync::Arc, thread, time::Duration};

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links: Vec<String> = tab
        .find_elements(".teaser__link")
        .context(".teaser__link")?
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
    tab.navigate_to("https://www.lemonde.fr/afrique/")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    if let Ok(cookie) = tab.find_element("button.gdpr-lmd-button") {
        cookie.click().context("clicking on cookie")?;
        thread::sleep(Duration::from_secs(1));
        trace!("clicked cookie");
    }

    for _ in 0..2 {
        tab.find_element("#js-more-teaser")
            .context("find_element #js-more-teaser")?
            .click()
            .context("click #js-more-teaser")?;
        thread::sleep(Duration::from_secs(1));
    }

    let links = get_articles_links(&tab).context(opts.provider.clone())?;
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
