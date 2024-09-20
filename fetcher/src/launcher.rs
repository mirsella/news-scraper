use std::{
    ffi::OsStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::sources::{GetNewsOpts, SeenLink, SourceFn};
use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::{error, info};
use shared::{config::Config, Telegram, News};
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

/// this is because the headless chrome crate doesn't support using multiple tab at the same time
/// if we want some parallelism, we need to spawn a whole new browser for each source
fn new_browser(headless: bool) -> Browser {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .window_size(Some((1920, 1080)))
            .headless(headless)
            .devtools(false)
            .args(vec![OsStr::new("--blink-settings=imagesEnabled=false")])
            .idle_browser_timeout(Duration::from_secs(120))
            .sandbox(false)
            .build()
            .unwrap(),
    )
    .unwrap();
    browser
}

pub fn init(
    config: &Config,
    sources: Vec<&'static (&'static str, SourceFn)>,
    seen_links: Arc<RwLock<Vec<SeenLink>>>,
    telegram: Arc<Telegram>,
) -> Receiver<anyhow::Result<News>> {
    let config = Arc::new(config.clone());
    let (tx, rx) = channel(500);
    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<()>>> = FuturesUnordered::new();
    let mut sources: Vec<_> = sources
        .into_iter()
        .map(|(s, f)| ((*s).to_string(), f))
        .collect();

    while futures.len() < config.chrome_concurrent.unwrap_or(4) {
        match sources.pop() {
            Some(source) => {
                info!("spawning {}", source.0);
                let opts = GetNewsOpts {
                    browser: new_browser(config.chrome_headless.unwrap_or(true)),
                    tx: tx.clone(),
                    seen_links: seen_links.clone(),
                    provider: source.0.to_string(),
                };
                futures.push(spawn_blocking(move || source.1(opts).context(source.0)));
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
                Some(source) => {
                    info!("spawning {}", source.0);
                    let opts = GetNewsOpts {
                        browser: new_browser(config.chrome_headless.unwrap_or(true)),
                        tx: tx.clone(),
                        seen_links: seen_links.clone(),
                        provider: source.0.to_string(),
                    };
                    futures.push(spawn_blocking(move || source.1(opts).context(source.0)));
                }
                None => break,
            }
        }
    });
    rx
}
