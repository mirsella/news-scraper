use super::{GetNewsOpts, News};
use anyhow::bail;
use anyhow::{anyhow, Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace};
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
    let mut links = Vec::with_capacity(articles.len());
    for article in articles {
        if let Some(attrs) = article.get_attributes().context("getting attributes")? {
            for i in 0..attrs.len() {
                if attrs[i] == "href" {
                    if let Some(link) = attrs.get(i + 1) {
                        links.push(link.clone());
                    }
                }
            }
        }
    }
    if links.is_empty() {
        return Err(anyhow!("didn't found any links"));
    }
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version()?.user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!("https://www.francetvinfo.fr/{}/", category))
            .context("navigate_to")?;
        tab.wait_until_navigated().context("wait_until_navigated")?;
        let links = get_articles_links(&tab).context("francetvinfo")?;
        trace!("found {} links on {category}", links.len());
        if links.is_empty() {
            bail!("no links found");
        }
        for url in links {
            let url = format!("https://www.francetvinfo.fr{}", &url);
            if opts.is_seen(&url) {
                continue;
            }

            let res = super::fetch_article(&url);
            let payload = match res {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: opts.provider.clone(),
                    date: res
                        .published
                        .parse()
                        .unwrap_or_else(|_| chrono::Local::now()),
                    body: res.content,
                    link: url,
                    ..Default::default()
                }),
                Err(err) => {
                    debug!("fetch_article: {err}");
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
