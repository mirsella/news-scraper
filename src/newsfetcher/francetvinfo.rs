use super::News;
use anyhow::Result;
use headless_chrome::Tab;
use std::sync::Arc;

pub fn get_news(_tab: Arc<Tab>) -> Result<Vec<News>> {
    let n = News {
        time: chrono::Utc::now(),
        ..News::default()
    };
    Ok(vec![n])
}
