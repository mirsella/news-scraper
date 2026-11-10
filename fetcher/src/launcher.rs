use std::{
    ffi::OsStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::sources::{GetNewsOpts, SourceFn};
use anyhow::Context;
use futures::{StreamExt, stream::FuturesUnordered};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::{error, info};
use shared::{News, Telegram, config::Config};
use tokio::{
    sync::mpsc::{Receiver, channel},
    task::{JoinHandle, spawn_blocking},
};

/// this is because the headless chrome crate doesn't support using multiple tab at the same time
/// if we want some parallelism, we need to spawn a whole new browser for each source
fn new_browser(headless: bool) -> Browser {
    Browser::new(
        LaunchOptionsBuilder::default()
            .window_size(Some((1920, 1080)))
            .headless(headless)
            .devtools(false)
            .args(vec![
                OsStr::new("--blink-settings=imagesEnabled=false"),
                OsStr::new("--disable-blink-features=AutomationControlled"),
                OsStr::new("--lang=fr-FR"),
            ])
            .idle_browser_timeout(Duration::from_secs(120))
            .sandbox(false)
            .build()
            .unwrap(),
    )
    .unwrap()
}

pub fn init(
    config: &Config,
    sources: Vec<&'static (&'static str, SourceFn)>,
    seen_links: Arc<RwLock<Vec<String>>>,
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
            };
        }
    });
    rx
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires a local Chromium installation"]
    fn headless_browser_can_read_a_page() -> anyhow::Result<()> {
        let browser = super::new_browser(true);
        let tab = browser.new_tab()?;
        tab.navigate_to(
            "data:text/html,<html><body><h1>Browser dependency smoke test</h1></body></html>",
        )?
        .wait_until_navigated()?;
        anyhow::ensure!(
            tab.wait_for_element("h1")?.get_inner_text()? == "Browser dependency smoke test"
        );
        Ok(())
    }
}
