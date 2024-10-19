pub mod config;
pub mod db_news;
pub mod telegram;
use chrono::{DateTime, Local};
pub use config::Config;
pub use db_news::DbNews;
pub use telegram::Telegram;

#[derive(Debug, Clone, Default)]
pub struct News {
    pub provider: String,
    pub date: DateTime<Local>,
    pub title: String,
    pub caption: String,
    pub body: String,
    pub link: String,
    pub tags: Vec<String>,
}

#[must_use]
pub fn sanitize_html(html: &str) -> String {
    let tags = maplit::hashset![
        "b", "i", "u", "em", "strong", "strike", "code", "hr", "br", "div", "table", "thead",
        "caption", "tbody", "tr", "th", "td", "p", "a", "img", "h1", "h2", "h3", "h4", "h5", "h6",
        "section"
    ];
    let allowed_attributes = ["href", "title", "src", "alt", "colspan", "style"];
    ammonia::Builder::new()
        .tags(tags)
        .link_rel(Some("noopener noreferrer"))
        .add_generic_attributes(&allowed_attributes)
        .clean(html)
        .to_string()
}

#[must_use]
pub fn extract_clean_text(html: &str) -> String {
    let s = nanohtml2text::html2text(html);
    let re = regex::Regex::new(r"\(?https?://[^\s]+").unwrap();
    let s = re.replace_all(&s, " ").to_string();
    let s = s
        .split_whitespace()
        .map(|s| s.trim().replace('\n', ""))
        .collect::<Vec<String>>()
        .join(" ");
    s
}
