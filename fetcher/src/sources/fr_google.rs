#![allow(dead_code)] // adapter intentionally unregistered while the site blocks automated access
use super::GetNewsOpts;
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{error, trace};
use shared::News;
use std::sync::Arc;
use std::time::Duration;

const KEYWORDS: [&str; 4] = ["bonne nouvelle", "joie", "optimisme", "entraide"];

fn dismiss_consent(tab: &Arc<Tab>) -> Result<()> {
    if !tab.get_url().contains("consent.google.com") {
        return Ok(());
    }
    let _ = tab.evaluate(
        r#"(() => {
            let clicked = false;
            for (const button of document.querySelectorAll('button')) {
                if ((button.innerText || '').includes('Tout refuser')) {
                    button.click();
                    clicked = true;
                }
            }
            return clicked;
        })()"#,
        false,
    );
    std::thread::sleep(Duration::from_secs(2));
    Ok(())
}

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let parent = tab
        .find_element("div[data-async-context^='query:']")
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
    let tab = opts.new_tab()?;
    tab.set_default_timeout(Duration::from_secs(30));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for keyword in KEYWORDS {
        trace!("checking out keyword {keyword}");
        tab.navigate_to(&format!(
            "https://www.google.com/search?q={keyword}&tbm=nws&tbs=qdr%3Ad"
        ))
        .context("navigate_to")?;
        tab.wait_until_navigated().context("wait_until_navigated")?;
        dismiss_consent(&tab)?;
        if tab.get_url().contains("consent.google.com") {
            tab.navigate_to(&format!(
                "https://www.google.com/search?q={keyword}&tbm=nws&tbs=qdr%3Ad"
            ))
            .context("navigate_to after consent")?;
            tab.wait_until_navigated()
                .context("wait_until_navigated after consent")?;
            dismiss_consent(&tab)?;
        }
        tab.wait_for_element("#center_col")
            .context("waiting on #center_col")?;

        let links = get_articles_links(&tab).context("google")?;
        trace!("found {} links on {keyword}", links.len());
        if links.is_empty() {
            bail!("no links found");
        }
        for url in links {
            if opts.is_seen(&url) {
                continue;
            }

            let mut res = super::fetch_article_with_browser_fallback(&opts.browser, &url);
            if let Err(err) = res {
                log::warn!("fetch_article on {url}: {err:?}");
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
                    provider: opts.provider.clone(),
                    date: res.published,
                    body: res.content,
                    link: url,
                    ..Default::default()
                }),
                Err(err) => {
                    log::warn!("parse_article on {url}: {err:?}");
                    continue;
                }
            };
            if let Err(e) = opts.tx.blocking_send(payload) {
                error!("blocking_send: {e}");
                break;
            };
        }
    }
    Ok(())
}
