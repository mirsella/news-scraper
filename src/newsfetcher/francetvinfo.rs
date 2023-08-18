use super::News;
use headless_chrome::Tab;
use std::sync::Arc;

pub fn get_news(_tab: Arc<Tab>) -> anyhow::Result<Vec<crate::newsfetcher::News>> {
    Ok(vec![News {
        title: "test".to_string(),
        description: "test".to_string(),
        link: "test".to_string(),
    }])
}
