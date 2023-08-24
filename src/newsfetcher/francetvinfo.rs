use std::sync::Arc;

use super::News;
use anyhow::{anyhow, Context, Result};
use headless_chrome::Tab;
use log::info;

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

fn get_info_on_article(tab: &Arc<Tab>, url: &str) -> Result<News> {
    tab.navigate_to(url)?;
    tab.wait_for_element(".c-body")?;
    let texts = tab
        .find_elements(
            ".c-body p, .c-body h1, .c-body h2, .c-body h3, .c-body h4, .c-body h5, .c-body h6",
        )
        .context("find_elements on .c-body")?;
    let body: String = texts.iter().map(|text| text.value.as_str()).collect();
    let time = tab
        .find_element(".publication-date__published > time")
        .context("find_element on publication-date__published")?
        .attributes
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
            .value,
        description: tab
            .find_element(".c-chapo")
            .context("find_element on .c-chapo")?
            .value,
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
    }
    if links.is_empty() {
        return Err(anyhow!("didn't found any links"));
    }
    Ok(links)
}

pub fn get_news(tab: Arc<Tab>) -> Result<Vec<News>> {
    let mut news = Vec::with_capacity(21 * CATEGORIES.len()); // about 21 articles by categorie
    for categorie in CATEGORIES {
        tab.navigate_to(&format!("https://www.francetvinfo.fr/{}/", categorie))?;
        tab.wait_until_navigated()?;
        if let Ok(cookies) = tab.find_element_by_xpath("#didomi-notice-agree-button") {
            cookies.click()?;
        }
        let links = get_articles_links(&tab)?;
        for link in links {
            news.push(
                get_info_on_article(&tab, &format!("https://www.francetvinfo.fr/{}/", link))
                    .context(link)?,
            );
        }
    }
    if news.is_empty() {
        return Err(anyhow!("didn't found any news"));
    }
    info!("francetvinfo: {} news found", news.len());
    Ok(news)
}
