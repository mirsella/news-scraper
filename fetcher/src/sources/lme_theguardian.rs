use super::GetNewsOpts;
use crate::sources::fetch_article;
use anyhow::{bail, Context, Result};
use headless_chrome::Tab;
use log::info;
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let allowed_path = ["/travel", "/for-the-stay", "/lifeandstyle", "/artanddesign"];
    Ok(tab
        .find_element("#maincontent")
        .context("find_elements #maincontent")?
        .find_elements("a[href]")
        .context("find_elements a[href]")?
        .iter()
        .filter_map(|el| {
            let mut link = el.get_attribute_value("href").unwrap().expect("a href");
            if !allowed_path.iter().any(|path| link.starts_with(path))
                || link.starts_with("/travel/all")
            {
                return None;
            }
            link.insert_str(0, "https://www.theguardian.com");
            Some(link)
        })
        .collect())
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.theguardian.com/uk/travel")
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
        let tags: Vec<_> = ["travel", "lemediaexperience"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let payload = match fetch_article(&url) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: opts.provider.clone(),
                tags,
date: res.published,
                body: res.content,
                link: url,
            }),
            Err(err) => {
                log::warn!("fetch_article on {url}: {err}");
                continue;
            }
        };
        opts.tx.blocking_send(payload)?;
    }
    Ok(())
}
