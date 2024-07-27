use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, info, trace};
use std::{sync::Arc, thread, time::Duration};

const BLACKLIST: &[&str] = &["ouestfrance-auto", "ouestfrance-immo", "ouestfrance-emploi"];
fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links: Vec<String> = tab
        .find_elements(".titre-lien")
        .expect(".titre-lien")
        .iter()
        .map(|el| el.get_attribute_value("href").unwrap().expect("no href ??"))
        .filter(|url| !BLACKLIST.iter().any(|b| url.contains(b)))
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version()?.user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.ouest-france.fr")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    if let Ok(cookie) = tab.find_element("#didomi-notice-agree-button") {
        cookie.click().context("clicking on cookie")?;
        thread::sleep(Duration::from_secs(1));
        trace!("clicked cookie");
    }

    let links = get_articles_links(&tab).context("ouest-france")?;
    info!("found {} articles", links.len());
    assert!(!links.is_empty());
    for url in links {
        if opts.seen_urls.read().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.write().unwrap().push(url.clone());

        let res = super::fetch_article(&url);
        let payload = match res {
            Ok(res) => Ok(News {
                tags: vec!["france".to_string()],
                title: res.title,
                caption: res.description,
                provider: "ouest-france".to_string(),
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
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
