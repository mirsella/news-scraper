use std::sync::Arc;

use super::News;
use anyhow::{anyhow, Context, Result};
use headless_chrome::Tab;
use log::{info, warn};

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

fn get_info_on_article(tab: &Arc<Tab>) -> Option<News> {
    Some(News::default())
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
    }
    if links.is_empty() {
        return Err(anyhow!("didn't found any links"));
    }
    Ok(links)
}

pub fn get_news(tab: Arc<Tab>) -> Result<Vec<News>> {
    let mut news = Vec::new();
    for categorie in CATEGORIES {
        tab.navigate_to(&format!("https://www.francetvinfo.fr/{}/", categorie))?;
        tab.wait_until_navigated()?;
        if let Ok(cookies) = tab.find_element_by_xpath("#didomi-notice-agree-button") {
            cookies.click()?;
        }
        let links = get_articles_links(&tab)?;
        info!("Found {} articles in {}", links.len(), categorie);
        for link in links {
            tab.navigate_to(&format!("https://www.francetvinfo.fr/{}/", link))?;
            tab.wait_until_navigated()?;
            if let Some(new) = get_info_on_article(&tab) {
                news.push(new);
            }
        }
    }
    Ok(vec![News::default()])
}
