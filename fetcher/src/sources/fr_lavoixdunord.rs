use super::{absolute_url, dedupe_links, parse_url_with_browser, GetNewsOpts, News};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error};
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = dedupe_links(
        tab.find_elements(".r-direct--link")?
            .iter()
            .filter_map(|el| {
                let href = el.get_attribute_value("href").unwrap()?;
                Some(absolute_url("https://www.lavoixdunord.fr", &href))
            }),
    );
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version()?.user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.lavoixdunord.fr/fil-info")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    let links = get_articles_links(&tab).context("lavoixdunord get_articles_links")?;
    if links.is_empty() {
        bail!("no links found");
    }
    for url in links {
        if opts.is_seen(&url) {
            continue;
        }
        let payload = match parse_url_with_browser(&tab, &url) {
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
                debug!("fetch_article on {url}: {err:?}");
                continue;
            }
        };
        if let Err(e) = opts.tx.blocking_send(payload) {
            error!("blocking_send: {e}");
            break;
        };
    }
    Ok(())
}
