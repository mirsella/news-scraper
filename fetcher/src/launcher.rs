use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use crate::sources::{GetNewsOpts, SOURCES};
use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use log::{error, info};
use shared::{config::Config, Telegram, *};
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

pub fn init(
    config: &Config,
    enabled: Option<Vec<String>>,
    seen_urls: Arc<Mutex<Vec<String>>>,
    telegram: Arc<Telegram>,
) -> Receiver<anyhow::Result<News>> {
    let config: Config = config.to_owned();
    let (tx, rx) = channel(500);

    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<()>>> = FuturesUnordered::new();
    let mut sources: Vec<_> = match enabled {
        Some(enabled) => SOURCES
            .iter()
            .filter(|s| enabled.contains(&s.0.to_string()))
            .collect(),
        None => SOURCES.iter().collect(),
    };

    let opts = GetNewsOpts {
        config: config.clone(),
        tx: tx.clone(),
        seen_urls,
    };
    while futures.len() < config.chrome_concurrent.unwrap_or(4) {
        match sources.pop() {
            Some(fetch) => {
                info!("spawning {}", fetch.0);
                let opts = opts.clone();
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
                    let source = e.source();
                    error!("thread panicked, source: {source:#?}, {e}");
                    if let Err(e) = telegram.send(format!(
                        "fetcher: thread panicked, source: {source:#?}, {e}"
                    )) {
                        error!("TelegramError: {}", e);
                    }
                    continue;
                }
                _ => (),
            };
            match sources.pop() {
                Some(fetch) => {
                    info!("spawning {}", fetch.0);
                    let opts = opts.clone();
                    futures.push(spawn_blocking(move || fetch.1(opts).context(fetch.0)));
                }
                None => break,
            }
        }
    });
    rx
}
