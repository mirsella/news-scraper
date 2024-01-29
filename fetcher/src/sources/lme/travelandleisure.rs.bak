use super::GetNewsOpts;
use crate::sources::parse_article;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, info, trace};
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    Ok(tab
        .find_elements("a[id^='mntl-card-list-items']")
        .context("finding a[id^='mntl-card-list-items']")?
        .iter()
        .map(|el| el.get_attribute_value("href").unwrap().expect("a href"))
        .collect())
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.travelandleisure.com/trip-ideas")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;
    let links = get_articles_links(&tab).context("get_articles_links")?;
    info!("found {} articles", links.len());
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());
        let tags: Vec<_> = ["trip-ideas", "lemediaexperience"]
            .into_iter()
            .map(str::to_string)
            .collect();
        tab.navigate_to(&url)
            .context("navigate_to url")?
            .wait_until_navigated()
            .context("wait_until_navigated url")?;

        let imgs_els = &tab
            .find_elements("img.primary-image__image, img[id^='mntl-sc-block-image'")
            .context("find_elements imgs_els")?;
        let imgs = imgs_els.iter().fold(String::new(), |mut s, el| {
            let src = match el.get_attribute_value("src").unwrap() {
                Some(src) => src,
                _ => el
                    .get_attribute_value("data-src")
                    .unwrap()
                    .expect("at least data-src"),
            };
            s += &format!("<img src='{src}' />");
            s
        });
        let body = tab.get_content()?;
        let payload = match parse_article(&body) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: "lme::travelandleisure".to_string(),
                tags,
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
                body: imgs.clone() + &res.content,
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
