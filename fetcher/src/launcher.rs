use std::{
    ffi::OsStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::sources::{GetNewsOpts, SourceFn};
use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::{error, info};
use shared::{config::Config, Telegram, *};
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

pub fn init(
    config: &Config,
    sources: Vec<&'static (&'static str, SourceFn)>,
    seen_urls: Arc<RwLock<Vec<String>>>,
    telegram: Arc<Telegram>,
) -> Receiver<anyhow::Result<News>> {
    let config = Arc::new(config.clone());
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .window_size(Some((1920, 1080)))
            .headless(config.chrome_headless.unwrap_or(true))
            .devtools(false)
            .user_data_dir(config.chrome_data_dir.clone())
            .args(vec![OsStr::new("--blink-settings=imagesEnabled=false")])
            .idle_browser_timeout(Duration::from_secs(60))
            .sandbox(false)
            .build()
            .unwrap(),
    )
    .unwrap();
    let (tx, rx) = channel(500);
    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<()>>> = FuturesUnordered::new();
    let mut sources: Vec<_> = sources
        .into_iter()
        .map(|(s, f)| (s.to_string(), f))
        .collect();

    while futures.len() < config.chrome_concurrent.unwrap_or(4) {
        match sources.pop() {
            Some(fetch) => {
                info!("spawning {}", fetch.0);
                let opts = GetNewsOpts {
                    browser: browser.clone(),
                    tx: tx.clone(),
                    seen_urls: seen_urls.clone(),
                };
                futures.push(spawn_blocking(move || fetch.1(opts).context(fetch.0)));
            }
            None => break,
        }
    }
    tokio::spawn(async move {
        while let Some(result) = futures.next().await {
            match result {
                Ok(Err(e)) => tx.send(Err(e)).await.unwrap(),
                Err(e) => {
                    error!("thread panicked: {e}");
                    if let Err(e) = telegram.send(format!("fetcher: thread panicked: {e}")) {
                        error!("TelegramError: {}", e);
                    }
                    continue;
                }
                _ => (),
            };
            match sources.pop() {
                Some(fetch) => {
                    info!("spawning {}", fetch.0);
                    let opts = GetNewsOpts {
                        browser: browser.clone(),
                        tx: tx.clone(),
                        seen_urls: seen_urls.clone(),
                    };
                    futures.push(spawn_blocking(move || fetch.1(opts).context(fetch.0)));
                }
                None => break,
            }
        }
    });
    rx
}
