use super::{absolute_url, dedupe_links, GetNewsOpts};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{info, trace};
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    Ok(dedupe_links(
        tab.find_elements("a[href*='/voyage/'], a[href*='/environnement/'], a[href*='/histoire/']")
            .context("finding articles links")?
            .iter()
            .filter_map(|a| {
                let link = a.get_attribute_value("href").unwrap()?;
                if link.contains("contenu-sponsorise")
                    || link.contains("/thematique/")
                    || link.ends_with("/destinations")
                    || link.ends_with("/voyage")
                    || link.ends_with("/environnement")
                    || link.ends_with("/histoire")
                {
                    return None;
                }
                Some(absolute_url("https://www.nationalgeographic.fr", &link))
            }),
    ))
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for category in ["voyage", "environnement", "histoire"] {
        trace!("checking out category {}", category);
        tab.navigate_to(&format!("https://www.nationalgeographic.fr/{category}"))
            .context("navigate_to")?
            .wait_until_navigated()
            .context("wait_until_navigated")?;
        let links = get_articles_links(&tab).context("get_articles_links")?;
        info!("found {} articles", links.len());
        if links.is_empty() {
            bail!("no links found");
        }
        for url in links {
            if opts.is_seen(&url) {
                continue;
            }
            let payload = match super::fetch_article_with_browser_fallback(&opts.browser, &url) {
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
                    log::warn!("fetch_article on {url}: {err:?}");
                    continue;
                }
            };
            opts.tx.blocking_send(payload)?;
        }
    }
    Ok(())
}
