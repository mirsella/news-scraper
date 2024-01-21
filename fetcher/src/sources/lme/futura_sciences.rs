use super::GetNewsOpts;
use crate::sources::parse_article;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, info, trace};
use shared::News;
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = tab
        .find_elements(".article-card-box, .keen-slider__slide")
        .context("finding .article-card-box, .keen-slider__slide")?
        .iter()
        .filter_map(|a| {
            if let Some(mut link) = a.get_attribute_value("href").unwrap() {
                link.insert_str(0, "https://futura-sciences.com");
                if link.contains("futura-sciences.com/live") {
                    return None;
                }
                return Some(link);
            }
            None
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.futura-sciences.com/planete/")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;
    let links = get_articles_links(&tab).context("lme::futura-sciences")?;
    info!("found {} articles", links.len());
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());

        let tags: Vec<_> = url
            .strip_prefix("https://futura-sciences.com/")
            .expect(&url)
            .split('/')
            .take(2)
            .chain(["lemediaexperience"].into_iter())
            .map(str::to_string)
            .collect();

        tab.navigate_to(&url)
            .context("navigate_to url")?
            .wait_until_navigated()
            .context("wait_until_navigated url")?;
        let body = tab.get_content()?;

        tab.evaluate(
            "document.querySelectorAll('.article-sidebar, .relative, .bottom2').forEach(e => e.remove())",
            false,
        )?;

        let mut imgs_els = tab
            .find_element(".article-content")
            .context("find_element .article-content")?
            .find_elements("img")
            .context("find_elements .article-content img")?;
        imgs_els.push(
            tab.find_element(".article-hero-image")
                .context("find_elements .article-hero-image")?
                .find_element("img")
                .context("find_element article-hero-image img")?,
        );
        let imgs = imgs_els.iter().fold(String::new(), |mut s, el| {
            let src = el.get_attribute_value("src").unwrap().expect("a src");
            s += &format!("<img src='{src}' />");
            s
        });
        let payload = match parse_article(&body) {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: "lme::futura-sciences".to_string(),
                tags,
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
                body: imgs + &res.content,
                link: url,
            }),
            Err(err) => {
                debug!("fetch_article: {}", err);
                continue;
            }
        };
        opts.tx.blocking_send(payload)?;
    }
    Ok(())
}
