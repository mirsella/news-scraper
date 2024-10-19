use super::GetNewsOpts;
use crate::sources::parse_article;
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{info, trace};
use shared::News;
use std::{sync::Arc, thread, time::Duration};

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    Ok(tab
        .find_element("#hub-page-first-page-content")
        .context("finding .hub-page-first-page-content")?
        .find_elements("a[title][href]")
        .context("finding a[title][href]")?
        .iter()
        .filter_map(|a| {
            let mut link = a.get_attribute_value("href").unwrap().expect("a href");
            link.insert_str(0, "https://www.nationalgeographic.fr");
            if link.contains("contenu-sponsorise") {
                return None;
            }
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
    for category in ["voyage", "environnement"] {
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
            let tags: Vec<_> = [category, "lemediaexperience"]
                .into_iter()
                .map(str::to_string)
                .collect();
            tab.navigate_to(&url)
                .context(format!("navigate_to {url}"))?
                .wait_until_navigated()
                .context("wait_until_navigated url")?;

            tab.evaluate("setInterval(() => window.scrollBy(0, 1000), 50)", false)?;
            thread::sleep(Duration::from_secs(1));
            tab.evaluate(
                "document.querySelectorAll('.css-1082emh').forEach(e => e.remove())",
                false,
            )?;
            let imgs_els = &tab
                .find_element("article")
                .context("find_element article")?
                .find_elements("img")
                .context("find_elements img")?;
            let imgs = imgs_els.iter().fold(String::new(), |mut s, el| {
                let src = el.get_attribute_value("src").unwrap().expect("a src");
                s += &format!("<img src='{src}' />");
                s
            });
            let body = tab.get_content()?;
            let payload = match parse_article(&body) {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: opts.provider.clone(),
                    tags,
                    date: res
                        .published
                        .parse()
                        .unwrap_or_else(|_| chrono::Local::now()),
                    body: imgs.clone() + &res.content,
                    link: url,
                }),
                Err(err) => {
                    log::warn!("parse_article: {err}");
                    continue;
                }
            };
            opts.tx.blocking_send(payload)?;
        }
    }
    Ok(())
}
