use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use chrono::Utc;
use headless_chrome::{Element, Tab};
use log::{debug, error, trace, warn};
use std::{sync::Arc, thread, time::Duration};

fn isvalidpost(el: &Element) -> bool {
    // skip ads
    if el.find_element("shreddit-dynamic-ad-link").is_ok() {
        return false;
    };

    // skip posts older than 2 days
    // let ts = match el.find_element("time") {
    //     Ok(el) => el,
    //     Err(_) => return false,
    // };
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
    let redditposts = tab.find_elements("shreddit-post")?;
    let goodposts = redditposts.into_iter().filter(isvalidpost);
    let links: Vec<String> = goodposts
        .map(|el| {
            el.get_attribute_value("content-href")
                .unwrap()
                .expect("reddig uplifingnews: get_attribute_value on content-href")
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.enable_stealth_mode()?;
    tab.navigate_to("https://www.reddit.com/r/UpliftingNews/new/")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;
    thread::sleep(Duration::from_secs(2));

    let links = get_articles_links(&tab).context("reddit-upliftingnews")?;
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());

        let mut res = super::fetch_article(&url);
        if let Err(err) = res {
            debug!("fetch_article: {}", err);
            if let Err(e) = tab.navigate_to(&url) {
                warn!("could not navigate to {url}: {e}");
                continue;
            };
            if let Err(e) = tab.wait_until_navigated().context("wait_until_navigated") {
                warn!("could not load {url}: {e}");
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
                provider: "reddit-upliftingnews".to_string(),
                date: res.published.parse().unwrap_or_else(|_| Utc::now()),
                body: res.content,
                link: url,
                ..Default::default()
            }),
            Err(err) => {
                debug!("parse_article: {}", err);
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
