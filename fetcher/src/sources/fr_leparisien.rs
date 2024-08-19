use super::{GetNewsOpts, News};
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace};
use std::{sync::Arc, thread};

const CATEGORIES: [&str; 7] = [
    "faits-divers",
    "politique",
    "economie",
    "societe",
    "sports",
    "culture-loisirs",
    "etudiant",
];

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let links = tab
        .find_elements("div[class^='story'] > a, *[class*='article__link']")
        .context("finding div[class^='story'] > a, *[class*='article__link']")?
        .iter()
        .map(|a| {
            let mut link = a
                .get_attribute_value("href")
                .expect("getting href")
                .expect("no href on article");
            if !link.starts_with("http") {
                link.insert_str(0, "http:");
            }
            link
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!("https://www.leparisien.fr/{category}"))
            .context("navigate_to")?;
        tab.wait_until_navigated()
            .context("category wait_until_navigated")?;

        if let Ok(cookies) = tab.find_element_by_xpath("//button[contains(text(), 'Accepter')]") {
            cookies.click().context("clicking on cookies")?;
            tab.wait_until_navigated()
                .context("cookies wait_until_navigated")?;
            thread::sleep(std::time::Duration::from_secs(1));
        };

        let links = get_articles_links(&tab).context("leparisien")?;
        trace!("found {} links on {category}", links.len());
        if links.is_empty() {
            return Err(anyhow::anyhow!("no links found"));
        }
        for url in links {
            if opts.is_seen(&url) {
                continue;
            }

            let res = super::fetch_article(&url);
            let payload = match res {
                Ok(res) => Ok(News {
                    tags: vec![category.to_string()],
                    title: res.title,
                    caption: res.description,
                    provider: opts.provider.clone(),
                    date: res
                        .published
                        .parse()
                        .unwrap_or_else(|_| chrono::Local::now()),
                    body: res.content,
                    link: url,
                }),
                Err(err) => {
                    debug!("fetch_article: {}", err);
                    continue;
                }
            };
            if let Err(e) = opts.tx.blocking_send(payload) {
                error!("blocking_send: {e}");
                break;
            }
        }
    }
    Ok(())
}
