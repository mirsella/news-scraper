automod::dir!("src/newsfetcher");

use std::{ffi::OsStr, sync::Arc, time::Duration};

use crate::config::Config;
use chrono::{DateTime, Utc};
use futures::{stream::FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use log::error;
use macros::vec_sources_fn;
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    task::{spawn_blocking, JoinHandle},
};

#[derive(Debug, Default, Clone)]
pub struct News {
    pub provider: String,
    pub time: DateTime<Utc>,
    pub title: String,
    pub description: String,
    pub body: String,
    pub link: String,
}

pub struct GetNewsOpts {
    pub tab: Arc<Tab>,
    pub tx: Sender<anyhow::Result<News>>,
    pub seen_urls: Vec<String>,
}
pub trait NewsFetcher {
    fn get_news(&self, opts: GetNewsOpts) -> anyhow::Result<()>;
    fn get_provider(&self) -> &'static str;
}

pub fn new(config: &Config, enabled: Vec<String>) -> Receiver<anyhow::Result<News>> {
    let (tx, rx) = channel(500);

    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(config.chrome.headless.unwrap_or(true))
            .user_data_dir(config.chrome.data_dir.clone())
            .args(vec![OsStr::new("--blink-settings=imagesEnabled=false")])
            .idle_browser_timeout(Duration::from_secs(60))
            .build()
            .unwrap(),
    )
    .unwrap();

    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<()>>> = FuturesUnordered::new();
    let mut sources = vec_sources_fn!("src/newsfetcher", "Fetcher");

    for _ in 0..config.chrome.concurrent_tabs.unwrap_or(10) {
        while let Some(fetch) = sources.pop() {
            if enabled.contains(&fetch.get_provider().into()) || enabled.is_empty() {
                let opts = GetNewsOpts {
                    tab: browser.new_tab().unwrap(),
                    tx: tx.clone(),
                    // TODO: get in the db the latest urls for this source
                    seen_urls: vec![],
                };
                opts.tab.enable_stealth_mode().unwrap();
                futures.push(spawn_blocking(move || fetch.get_news(opts)));
                break;
            }
        }
    }
    tokio::spawn(async move {
        while let Some(result) = futures.next().await {
            match result {
                Ok(Err(e)) => tx.send(Err(e)).await.unwrap(),
                Err(e) => {
                    error!("JoinError: {:?}", e);
                    continue;
                }
                _ => (),
            };
            while let Some(fetch) = sources.pop() {
                if enabled.contains(&fetch.get_provider().into()) || enabled.is_empty() {
                    let opts = GetNewsOpts {
                        tab: browser.new_tab().unwrap(),
                        tx: tx.clone(),
                        // TODO: get in the db the latest urls for this source
                        seen_urls: vec![],
                    };
                    opts.tab.enable_stealth_mode().unwrap();
                    futures.push(spawn_blocking(move || fetch.get_news(opts)));
                    break;
                }
            }
        }
    });
    rx
}
