use std::{ffi::OsStr, time::Duration};

use crate::sources::{GetNewsOpts, SOURCES};
use futures::{stream::FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::error;
use shared::*;
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

pub fn new(config: &Config, enabled: Vec<String>) -> Receiver<anyhow::Result<News>> {
    let (tx, rx) = channel(500);

    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(config.chrome_headless.unwrap_or(true))
            .user_data_dir(config.chrome_data_dir.clone())
            .args(vec![OsStr::new("--blink-settings=imagesEnabled=false")])
            .idle_browser_timeout(Duration::from_secs(60))
            .sandbox(false)
            .build()
            .unwrap(),
    )
    .unwrap();

    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<()>>> = FuturesUnordered::new();
    let mut sources = SOURCES.to_vec();

    for _ in 0..config.chrome_concurrent_tabs.unwrap_or(10) {
        while let Some(fetch) = sources.pop() {
            if enabled.contains(&fetch.0.to_string()) || enabled.is_empty() {
                let opts = GetNewsOpts {
                    tab: browser.new_tab().unwrap(),
                    tx: tx.clone(),
                    // TODO: get in the db the latest urls for this source
                    seen_urls: vec![],
                };
                opts.tab.enable_stealth_mode().unwrap();
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
                    continue;
                }
                _ => (),
            };
            while let Some(fetch) = sources.pop() {
                if enabled.contains(&fetch.0.to_string()) || enabled.is_empty() {
                    let opts = GetNewsOpts {
                        tab: browser.new_tab().unwrap(),
                        tx: tx.clone(),
                        // TODO: get in the db the latest urls for this source
                        seen_urls: vec![],
                    };
                    opts.tab.enable_stealth_mode().unwrap();
                    futures.push(spawn_blocking(move || fetch.1(opts)));
                    break;
                }
            }
        }
    });
    rx
}
