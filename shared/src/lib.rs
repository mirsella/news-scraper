pub mod config;
pub mod db_news;
pub mod telegram;
use chrono::{DateTime, Utc};
pub use config::Config;
pub use db_news::DbNews;
pub use telegram::Telegram;

#[derive(Debug, Clone)]
pub struct News {
    pub provider: String,
    pub date: DateTime<Utc>,
    pub title: String,
    pub caption: String,
    pub body: String,
    pub link: String,
}
impl Default for News {
    fn default() -> Self {
        News {
            provider: "DefaultProvider".to_string(),
            date: Utc::now(), // use the current time as default
            title: "DefaultTitle".to_string(),
            caption: "DefaultCaption".to_string(),
            body: "DefaultBody".to_string(),
            link: "http://example.com".to_string(),
        }
    }
}

pub fn sanitize_html(html: &str) -> String {
    let tags = maplit::hashset![
        "b", "i", "u", "em", "strong", "strike", "code", "hr", "br", "div", "table", "thead",
        "caption", "tbody", "tr", "th", "td", "p", "a", "img", "h1", "h2", "h3", "h4", "h5", "h6",
        "section"
    ];
    let allowed_attributes = ["href", "title", "src", "alt", "colspan"];
    ammonia::Builder::new()
        .tags(tags)
        .link_rel(None)
        .add_generic_attributes(&allowed_attributes)
        .clean(html)
        .to_string()
}

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
