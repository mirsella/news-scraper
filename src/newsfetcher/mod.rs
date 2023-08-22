automod::dir!("src/newsfetcher");

use crate::config::Config;
use chrono::{DateTime, Utc};
use futures::{stream::FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::error;
use macros::vec_sources_fn;
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

#[derive(Debug, Default)]
pub struct News {
    pub provider: String,
    pub time: DateTime<Utc>,
    pub title: String,
    pub text: String,
    pub link: String,
}

pub fn new(config: &Config) -> Receiver<anyhow::Result<News>> {
    let (tx, rx) = channel(100);

    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(config.chrome.headless.unwrap_or(true))
            .user_data_dir(config.chrome.data_dir.clone())
            .build()
            .unwrap(),
    )
    .unwrap();

    let mut sources = vec_sources_fn!("src/newsfetcher", "get_news");
    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<Vec<News>>>> =
        FuturesUnordered::new();

    for _ in 0..config.chrome.concurrent_tabs.unwrap_or(10) {
        if let Some(fetch) = sources.pop() {
            let tab = browser.new_tab().unwrap();
            futures.push(spawn_blocking(move || {
                let res = fetch(tab.clone());
                tab.close_target().unwrap();
                res
            }));
        }
    }
    tokio::spawn(async move {
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(news)) => {
                    for new in news {
                        tx.send(Ok(new)).await.unwrap();
                    }
                }
                Ok(Err(e)) => tx.send(Err(e)).await.unwrap(),
                Err(e) => {
                    error!("JoinError: {:?}", e);
                    continue;
                }
            };
            if let Some(fetch) = sources.pop() {
                let tab = browser.new_tab().unwrap();
                futures.push(spawn_blocking(move || {
                    let res = fetch(tab.clone());
                    tab.close_target().unwrap();
                    res
                }));
            }
        }
    });
    rx
}
