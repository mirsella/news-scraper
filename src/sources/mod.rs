use crate::newsfetcher::News;
use headless_chrome::Tab;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
automod::dir!("src/sources");

pub struct GetNewsOpts {
    pub tab: Arc<Tab>,
    pub tx: Sender<anyhow::Result<News>>,
    pub seen_urls: Vec<String>,
}
type GetNewsFn = fn(GetNewsOpts) -> anyhow::Result<()>;

pub static SOURCES: [(&str, GetNewsFn); 1] = [("francetvinfo", francetvinfo::get_news)];
