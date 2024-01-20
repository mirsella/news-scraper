use super::GetNewsOpts;
use crate::sources::parse_article;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, info, trace};
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    Ok(tab
        .find_elements("h2.articlePreview-title > a")
        .context("finding h2.articlePreview-title > a")?
        .iter()
        .map(|a| a.get_attribute_value("href").unwrap().expect("a href"))
        .collect())
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.geo.fr/aventure")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;
    let links = get_articles_links(&tab).context("lme::geo")?;
    info!("found {} articles", links.len());
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());
        let tags: Vec<_> = ["adventure", "lemediaexperience"]
            .into_iter()
            .map(str::to_string)
            .collect();

        tab.navigate_to(&url)
            .context("navigate_to url")?
            .wait_until_navigated()
            .context("wait_until_navigated url")?;
        let body = tab.get_content()?;
        let payload = match parse_article(&body) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: "lme::geo".to_string(),
                tags,
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
                body: res.content,
                link: url,
            }),
            Err(err) => {
                debug!("parse_article: {}", err);
                continue;
            }
        };
        opts.tx.blocking_send(payload)?;
    }
    Ok(())
}
