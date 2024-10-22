use super::{GetNewsOpts, News};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{error, trace};
use std::sync::Arc;

const CATEGORIES: [&str; 9] = [
    "politique",
    "societe",
    "faits-divers",
    "sante",
    "economie",
    "monde",
    "culture",
    "sport",
    "environnement",
];

// number of links keep per category
const NUMBER_OF_ARTICLES_PER_CATEGORY: usize = 14;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let mut articles = tab
        .find_elements(
            ".card-article-m__link, .card-article-majeure__link, .card-article-l__link, .card-article-list-l__link, .card-article-list-s__link",
        )
        .context("gettings articles __links")?;
    articles.truncate(NUMBER_OF_ARTICLES_PER_CATEGORY);
    let links = articles
        .iter()
        .map(|el| el.get_attribute_value("href").unwrap().expect("no href ??"))
        .map(|mut link| {
            if !link.starts_with("http") {
                if !link.starts_with('/') {
                    link.insert(0, '/');
                }
                link.insert_str(0, "https://www.francetvinfo.fr");
            }
            link
        })
        .collect();
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version()?.user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!("https://www.francetvinfo.fr/{category}/"))
            .context("navigate_to")?;
        tab.wait_until_navigated().context("wait_until_navigated")?;
        let links = get_articles_links(&tab).context("francetvinfo")?;
        trace!("found {} links on {category}", links.len());
        if links.is_empty() {
            bail!("no links found");
        }
        for url in links {
            if opts.is_seen(&url) {
                continue;
            }

            let res = super::fetch_article(&url);
            let payload = match res {
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
            if let Err(e) = opts.tx.blocking_send(payload) {
                error!("blocking_send: {e}");
                break;
            }
        }
    }
    Ok(())
}
