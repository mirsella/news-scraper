use super::{GetNewsOpts, News};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::{Element, Tab};
use log::error;
use std::{sync::Arc, thread, time::Duration};

fn _isvalidpost(el: &Element) -> bool {
    // skip ads
    if el.find_element("shreddit-dynamic-ad-link").is_ok() {
        return false;
    };

    // skip posts older than 2 days
    // let ts = match el.find_element("time") {
    //     Ok(el) => el,
    //     Err(_) => return false,
    // let ts = match ts.get_attribute_value("datetime").unwrap() {
    //     Some(ts) => ts,
    //     None => return false,
    // };
    // let ts = ts.parse::<DateTime<Utc>>().unwrap();
    // if ts < Utc::now() - chrono::Duration::days(2) {
    //     return false;
    // }

    true
}

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let redditposts = tab.find_elements("shreddit-post  .a")?;
    let alllinks = redditposts
        .iter()
        .filter_map(|el| el.get_attribute_value("href").unwrap());
    let links = alllinks.filter(|link| link.starts_with("https"));
    Ok(links.collect())
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.reddit.com/r/UpliftingNews/new/")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;
    thread::sleep(Duration::from_secs(2));

    let links = get_articles_links(&tab).context("reddit-upliftingnews")?;
    if links.is_empty() {
        bail!("no links found");
    }
    for url in links {
        if opts.is_seen(&url) {
            continue;
        }

        let mut res = super::fetch_article(&url);
        if let Err(err) = res {
            log::warn!("fetch_article on {url}: {err}");
            if let Err(e) = tab.navigate_to(&url) {
                log::warn!("could not navigate to {url}: {e}");
                continue;
            };
            if let Err(e) = tab.wait_until_navigated().context("wait_until_navigated") {
                log::warn!("could not load {url}: {e}");
                continue;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let doc = tab
                .get_content()
                .context("reddit-upliftingnews: tab.get_content()")?;
            res = super::parse_article(&doc);
        }
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
                log::warn!("parse_article on {url}: {err}");
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
