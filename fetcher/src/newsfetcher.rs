use std::{
    ffi::OsStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::sources::{GetNewsOpts, SOURCES};
use futures::{stream::FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::{error, trace};
use shared::{config::Config, Telegram, *};
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

fn new_browser(config: &Config) -> Browser {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .window_size(Some((1920, 1080)))
            .headless(config.chrome_headless.unwrap_or(true))
            .user_data_dir(config.chrome_data_dir.clone())
            .args(vec![OsStr::new("--blink-settings=imagesEnabled=false")])
            .idle_browser_timeout(Duration::from_secs(60))
            .sandbox(false)
            .build()
            .unwrap(),
    )
    .unwrap();
    browser
}

pub fn new(
    config: &Config,
    enabled: Vec<String>,
    seen_urls: Arc<Mutex<Vec<String>>>,
    telegram: Telegram,
) -> Receiver<anyhow::Result<News>> {
    let config: Config = config.to_owned();
    let (tx, rx) = channel(500);

    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<()>>> = FuturesUnordered::new();
    let mut sources = SOURCES.to_vec();

    for _ in 0..config.chrome_concurrent.unwrap_or(4) {
        while let Some(fetch) = sources.pop() {
            if enabled.contains(&fetch.0.to_string()) || enabled.is_empty() {
                let opts = GetNewsOpts {
                    browser: new_browser(&config),
                    tx: tx.clone(),
                    seen_urls: seen_urls.clone(),
                };
                trace!("spawning {}", fetch.0);
                futures.push(spawn_blocking(move || fetch.1(opts)));
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
                    if let Err(e) = telegram.send(format!("JoinError: {:?}", e)) {
                        error!("TelegramError: {:?}", e);
                    }
                    continue;
                }
                _ => (),
            };
            while let Some(fetch) = sources.pop() {
                if enabled.contains(&fetch.0.to_string()) || enabled.is_empty() {
                    let opts = GetNewsOpts {
                        browser: new_browser(&config),
                        tx: tx.clone(),
                        seen_urls: seen_urls.clone(),
                    };
                    trace!("spawning {}", fetch.0);
                    futures.push(spawn_blocking(move || fetch.1(opts)));
                    break;
                }
            }
        }
    });
    rx
}
