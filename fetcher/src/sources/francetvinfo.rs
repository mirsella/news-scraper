use super::{GetNewsOpts, News};
use anyhow::{anyhow, Context, Result};
use headless_chrome::{Element, Tab};
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

fn _get_info_on_article(url: &str, tab: &Arc<Tab>) -> Result<News> {
    tab.navigate_to(url)?;
    tab.wait_for_elements(".c-body p, .c-body h2, .p-para")
        .context("waiting for .c-body child")?;
    if tab.find_element(".faq-highlight").is_ok() {
        return Err(anyhow!("found faq-highlight"));
    }
    let texts: Vec<Element> = tab
        .find_elements(".c-body p, .c-body h2, .p-para")
        .context("find_elements on .c-body")?
        .into_iter()
        .filter(|e| {
            e.get_inner_text().map_or(false, |text| {
                !["LIRE AUSSI", "EDITO"].contains(&text.to_uppercase().as_str())
            })
        })
        .collect();
    let body: String = texts
        .iter()
        .filter_map(|text| text.get_inner_text().ok())
        .collect();
    let date = tab
        .find_element(".publication-date__published > time")
        .context("find_element on publication-date__published")?
        .get_attributes()?
        .ok_or(anyhow!("no attributes on time"))?
        .get(1)
        .ok_or(anyhow!("no second attributes for time"))?
        .parse()?;
    let new = News {
        link: tab.get_url(),
        provider: "francetvinfo".to_string(),
        title: tab
            .find_element(".c-title, h1[class$='__title']")
            .context("find_element on .c-title")?
            .get_inner_text()?,
        caption: tab
            .find_element(".c-chapo")
            .context("find_element on .c-chapo")?
            .get_inner_text()?,
        date,
        body,
    };
    Ok(new)
}

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
    let tab = opts.browser.new_tab()?;
    tab.enable_stealth_mode()?;
    for category in CATEGORIES {
        trace!("checking out category {category}");
        tab.navigate_to(&format!("https://www.francetvinfo.fr/{}/", category))
            .context("navigate_to")?;
        tab.wait_until_navigated().context("wait_until_navigated")?;
        if let Ok(cookies) = tab.find_element_by_xpath("#didomi-notice-agree-button") {
            cookies.click().context("clicking on cookies")?;
        }
        let links = get_articles_links(&tab)?;
        trace!("found {} links on {category}", links.len());
        for link in links {
            let url = format!("https://www.francetvinfo.fr/{}/", link);
            if opts.seen_urls.lock().unwrap().contains(&url) {
                trace!("already seen {url}");
                continue;
            }
            opts.seen_urls.lock().unwrap().push(url.clone());

            // let new = get_info_on_article(&url, &tab)
            //     .context(link);
            // if new
            //     .as_ref()
            //     .is_err_and(|e| e.to_string().contains("found faq-highlight"))
            // {
            //     println!("error: {:#?}", new);
            //     break;
            // }
            // let tx = tx.clone();
            // if let Err(e) = tx.blocking_send(new) {
            //     error!("blocking_send: {e:?}");
            //     break;
            // }

            let mut res = super::fetch_article(&url);
            if let Err(err) = res {
                debug!("fetch_article: {}", err);
                if tab.navigate_to(&url).is_err() {
                    continue;
                };
                if tab
                    .wait_for_elements(".c-body p, .c-body h2, .p-para")
                    .is_err()
                {
                    continue;
                }
                let doc = tab.get_content()?;
                res = super::parse_article(&doc);
            }
            let payload = match res {
                Ok(res) => Ok(News {
                    title: res.title,
                    caption: res.description,
                    provider: "francetvinfo".to_string(),
                    date: res.published.parse().unwrap_or_else(|_| chrono::Utc::now()),
                    body: res.content,
                    link: url,
                }),
                Err(err) => {
                    debug!("parse_article: {}", err);
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
