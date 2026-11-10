use super::{ApiResponse, GetNewsOpts};
use anyhow::{bail, Context, Result};
use headless_chrome::Tab;
use log::{error, info, trace};
use shared::News;
use std::sync::Arc;
use std::time::Duration;

const KEYWORDS: [&str; 4] = ["bonne nouvelle", "joie", "optimisme", "entraide"];

fn dismiss_consent(tab: &Arc<Tab>) -> Result<()> {
    let clicked = tab
        .evaluate(
            r#"(() => {
                for (const button of document.querySelectorAll('button, div[role="button"]')) {
                    const text = (button.innerText || '').trim();
                    if (/^(Tout accepter|J'accepte|Accept all)$/i.test(text)) {
                        button.click();
                        return true;
                    }
                }
                return false;
            })()"#,
            false,
        )
        .ok()
        .and_then(|result| result.value)
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if clicked {
        trace!("accepted Google consent");
        std::thread::sleep(Duration::from_secs(2));
    }
    Ok(())
}

fn search(tab: &Arc<Tab>, keyword: &str) -> Result<()> {
    tab.navigate_to("https://www.google.com/")
        .context("navigate_to google homepage")?
        .wait_until_navigated()
        .context("wait_until_navigated google homepage")?;
    dismiss_consent(tab)?;

    let input = tab
        .find_element("textarea[name='q'], input[name='q']")
        .context("finding Google search box")?;
    input.click().context("clicking Google search box")?;
    tab.type_str(keyword).context("typing Google query")?;
    tab.press_key("Enter").context("submitting Google query")?;
    tab.wait_for_element("#center_col")
        .context("waiting for Google results")?;
    dismiss_consent(tab)?;

    if let Ok(news) = tab.find_element("a[href*='tbm=nws']") {
        news.click().context("clicking Google news tab")?;
        std::thread::sleep(Duration::from_secs(2));
        tab.wait_for_element("#center_col")
            .context("waiting for Google news results")?;
    }
    Ok(())
}

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let parent = tab
        .find_element("div[data-async-context^='query:']")
        .context("finding parent of articles")?;

    let links: Vec<String> = parent
        .find_elements("a[href]")
        .context("finding <a> on parent")?
        .iter()
        .filter_map(|a| a.get_attribute_value("href").ok().flatten())
        .filter(|href| {
            href.contains("/goto?url=")
                || href.contains("/url?q=")
                || (href.starts_with("http") && !href.contains("google."))
        })
        .map(|href| super::absolute_url("https://www.google.com/", &href))
        .collect();
    Ok(super::dedupe_links(links))
}

fn parse_result(tab: &Arc<Tab>, url: &str) -> Result<(String, ApiResponse)> {
    let article = super::parse_url_with_browser(tab, url)?;
    let final_url = tab.get_url();
    if final_url.contains("google.") || final_url.contains("consent.google") {
        bail!("Google result did not redirect to a publisher: {final_url}");
    }
    Ok((final_url, article))
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.new_tab()?;
    tab.set_default_timeout(Duration::from_secs(30));
    for keyword in KEYWORDS {
        trace!("checking out keyword {keyword}");
        search(&tab, keyword)?;

        let links = get_articles_links(&tab).context("google")?;
        info!("found {} links on {keyword}", links.len());
        if links.is_empty() {
            bail!("no links found for {keyword}");
        }
        for url in links {
            let (final_url, res) = match parse_result(&tab, &url) {
                Ok(result) => result,
                Err(err) => {
                    log::warn!("parse_article on {url}: {err:?}");
                    continue;
                }
            };
            if opts.is_seen(&final_url) {
                continue;
            }
            let payload = News {
                title: res.title,
                caption: res.description,
                provider: opts.provider.clone(),
                date: res.published,
                body: res.content,
                link: final_url,
                ..Default::default()
            };
            if let Err(e) = opts.tx.blocking_send(Ok(payload)) {
                error!("blocking_send: {e}");
                break;
            }
        }
    }
    Ok(())
}
