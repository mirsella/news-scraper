use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace};
use std::sync::Arc;

const KEYWORDS: [&str; 4] = ["bonne nouvelle", "joie", "optimisme", "entraide"];

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let parent = tab
        .find_element_by_xpath("/html/body/div[5]/div/div[11]/div/div[2]/div[2]/div/div/div/div")
        .context("finding parent of articles")?;

    let links = parent
        .find_elements("a")
        .context("finding <a> on parent")?
        .iter()
        .map(|a| {
            a.get_attribute_value("href")
                .expect("getting href")
                .expect("no href on article")
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let browser = opts.new_browser(false);
    let tab = browser.new_tab()?;
    let user_agent = browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for keyword in KEYWORDS {
        trace!("checking out keyword {keyword}");
        tab.navigate_to(&format!(
            "https://www.google.com/search?q={}&tbm=nws&tbs=qdr%3Ad",
            keyword
        ))
        .context("navigate_to")?;
        tab.wait_until_navigated().context("wait_until_navigated")?;
        if let Ok(cookies) = tab.find_element_by_xpath("//span[contains(text(), 'Tout refuser')]") {
            cookies.click().context("clicking on cookies")?;
            tab.wait_until_navigated()?;
        }
        tab.wait_for_element("#center_col")
            .context("waiting on #center_col")?;

        let links = get_articles_links(&tab).context("google")?;
        trace!("found {} links on {keyword}", links.len());
        for url in links {
            if opts.seen_urls.lock().unwrap().contains(&url) {
                trace!("already seen {url}");
                continue;
            }
            opts.seen_urls.lock().unwrap().push(url.clone());

            let mut res = super::fetch_article(&url);
            if let Err(err) = res {
                debug!("fetch_article: {}", err);
                if tab.navigate_to(&url).is_err() {
                    continue;
                };
                if tab
                    .wait_until_navigated()
                    .context("wait_until_navigated")
                    .is_err()
                {
                    continue;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
                let doc = tab.get_content().context("google: tab.get_content()")?;
                res = super::parse_article(&doc);
            }
            let payload = match res {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: "google".to_string(),
                    date: res.published.parse().unwrap_or_else(|_| chrono::Utc::now()),
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
    }
    Ok(())
}
