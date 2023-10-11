use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace};
use std::{sync::Arc, time::Instant};

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let time = Instant::now();
    let links: Vec<String> = tab
        .find_elements(".titre-lien")
        .expect(".titre-lien")
        .iter()
        .map(|el| el.get_attribute_value("href").unwrap().expect("no href ??"))
        .collect();
    debug!("get_articles_links took: {:#?}", time.elapsed());
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.ouest-france.fr/actualite-en-continu/archives/")
        .context("navigate_to")?;
    tab.wait_until_navigated().context("wait_until_navigated")?;

    let links = get_articles_links(&tab).context("ouest-france")?;
    for url in links {
        if opts.seen_urls.lock().unwrap().contains(&url) {
            trace!("already seen {url}");
            continue;
        }
        opts.seen_urls.lock().unwrap().push(url.clone());

        let res = super::fetch_article(&url);
        // if let Err(e) = res {
        //     trace!("fetch_article {url}: {}", e);
        //     tab.navigate_to(&url)
        //         .context("ouest-france navigate_to url")?;
        //     tab.wait_until_navigated()
        //         .context("ouest-france wait_until_navigated parse")?;
        //     let doc = tab.get_content().context("ouest-france get_content")?;
        //     res = super::parse_article(&doc);
        // }
        let payload = match res {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: "ouest-france".to_string(),
                date: res.published.parse().unwrap_or_else(|_| chrono::Utc::now()),
                body: res.content,
                link: url,
            }),
            Err(err) => {
                debug!("fetch_article {url}: {err}");
                continue;
            }
        };
        if let Err(e) = opts.tx.blocking_send(payload) {
            error!("blocking_send: {e}");
            break;
        }
    }
    Ok(())
}
