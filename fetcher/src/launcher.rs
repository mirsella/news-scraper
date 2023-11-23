use std::sync::{Arc, Mutex};

use crate::sources::{GetNewsOpts, SOURCES};
use futures::{stream::FuturesUnordered, StreamExt};
use log::{error, trace};
use shared::{config::Config, Telegram, *};
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

pub fn init(
    config: &Config,
    enabled: Vec<String>,
    seen_urls: Arc<Mutex<Vec<String>>>,
    telegram: Arc<Telegram>,
) -> Receiver<anyhow::Result<News>> {
    let config: Config = config.to_owned();
    let (tx, rx) = channel(500);

    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<()>>> = FuturesUnordered::new();
    let mut sources = SOURCES.to_vec();

    let opts = GetNewsOpts {
        config: config.clone(),
        tx: tx.clone(),
        seen_urls,
    };
    for _ in 0..config.chrome_concurrent.unwrap_or(4) {
        while let Some(fetch) = sources.pop() {
            if enabled.contains(&fetch.0.to_string()) || enabled.is_empty() {
                trace!("spawning {}", fetch.0);
                let opts = opts.clone();
                futures.push(spawn_blocking(move || fetch.1(opts)));
                break;
            }
        }
    }
    tokio::spawn(async move {
        while let Some(result) = futures.next().await {
            match result {
                Ok(Err(e)) => tx.send(Err(e)).await.unwrap(),
                Err(e) if e.is_panic() => {
                    let e = e.into_panic();
                    error!("thread paniced: {:?}", e);
                    if let Err(e) = telegram.send(format!("fetcher: thread paniced: {:?}", e)) {
                        error!("TelegramError: {}", e);
                    }
                    continue;
                }
                _ => (),
            };
            while let Some(fetch) = sources.pop() {
                if enabled.contains(&fetch.0.to_string()) || enabled.is_empty() {
                    trace!("spawning {}", fetch.0);
                    let opts = opts.clone();
                    futures.push(spawn_blocking(move || fetch.1(opts)));
                    break;
                }
            }
        }
    });
    rx
}
