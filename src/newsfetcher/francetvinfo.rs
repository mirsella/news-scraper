use super::{GetNewsOpts, News, NewsFetcher};
use anyhow::{anyhow, Context, Result};
use headless_chrome::{Element, Tab};
use log::error;
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

const NUMBER_OF_ARTICLES_PER_CATEGORY: usize = 13;

fn get_info_on_article(tab: &Arc<Tab>, url: &str) -> Result<News> {
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
    let time = tab
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
        description: tab
            .find_element(".c-chapo")
            .context("find_element on .c-chapo")?
            .get_inner_text()?,
        time,
        body,
    };
    Ok(new)
}

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    let articles = tab
        .find_elements(
            ".card-article-m__link, .card-article-majeure__link, .card-article-list-l__link, .card-article-l__link",
        )
        .context("gettings articles __links")?;
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
        if links.len() >= NUMBER_OF_ARTICLES_PER_CATEGORY {
            break;
        }
    }
    if links.is_empty() {
        return Err(anyhow!("didn't found any links"));
    }
    Ok(links)
}

pub struct Fetcher;
impl NewsFetcher for Fetcher {
    fn get_provider(&self) -> &'static str {
        "francetvinfo"
    }
    fn get_news(&self, opts: GetNewsOpts) -> Result<()> {
        let (tab, tx) = (opts.tab, opts.tx);
        let mut seen_urls: Vec<String> =
            Vec::with_capacity(CATEGORIES.len() * NUMBER_OF_ARTICLES_PER_CATEGORY);
        for categorie in CATEGORIES {
            tab.navigate_to(&format!("https://www.francetvinfo.fr/{}/", categorie))
                .context("francetvinfo navigate_to")?;
            tab.wait_until_navigated()
                .context("francetvinfo wait_until_navigated")?;
            if let Ok(cookies) = tab.find_element_by_xpath("#didomi-notice-agree-button") {
                cookies
                    .click()
                    .context("francetvinfo clicking on cookies")?;
            }
            let links = get_articles_links(&tab)?;
            for link in links {
                if seen_urls.contains(&link) {
                    error!("already seen {link}");
                    break;
                }
                seen_urls.push(link.clone());
                let new =
                    get_info_on_article(&tab, &format!("https://www.francetvinfo.fr/{}/", link))
                        .context(link);
                if new
                    .as_ref()
                    .is_err_and(|e| e.to_string().contains("found faq-highlight"))
                {
                    println!("error: {:#?}", new);
                    break;
                }
                let tx = tx.clone();
                if let Err(e) = tx.blocking_send(new) {
                    error!("blocking_send: {e:?}");
                    break;
                }
            }
        }
        Ok(())
    }
}
