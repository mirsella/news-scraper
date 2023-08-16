use crate::config::CONFIG;
use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::{debug, error};
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

#[derive(Debug)]
pub struct News {
    pub title: String,
}

pub fn new() -> Receiver<anyhow::Result<News>> {
    let (tx, rx) = channel(50);

    let mut urls = vec![
        "https://www.sciencesetavenir.fr/",
        "https://www.lemonde.fr/",
        "https://www.lefigaro.fr/",
        "https://www.liberation.fr/",
        "https://www.leparisien.fr/",
        "https://www.20minutes.fr/",
        "https://www.lci.fr/",
        "https://www.francetvinfo.fr/",
        "https://www.bfmtv.com/",
        "https://www.europe1.fr/",
        "https://www.rtl.fr/",
        "https://www.franceinter.fr/",
        "https://www.francebleu.fr/",
        "https://www.franceinfo.fr/",
    ];
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(CONFIG.headless)
            .build()
            .unwrap(),
    )
    .unwrap();
    let mut futures = FuturesUnordered::new();
    for _ in 0..CONFIG.concurrent_tabs {
        if let Some(url) = urls.pop() {
            futures.push(get_title(browser.clone(), url.to_string()));
        }
    }
    tokio::spawn(async move {
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(title)) => tx.send(Ok(News { title })),
                Ok(Err(e)) => tx.send(Err(e)),
                Err(e) => {
                    error!("JoinError: {:?}", e);
                    continue;
                }
            }
            .await
            .unwrap();
            if let Some(url) = urls.pop() {
                futures.push(get_title(browser.clone(), url.to_string()));
            }
        }
    });
    rx
}

fn get_title(browser: Browser, url: String) -> JoinHandle<anyhow::Result<String>> {
    spawn_blocking(move || -> anyhow::Result<String> {
        let tab = browser.new_tab()?;
        tab.enable_stealth_mode()?;
        tab.navigate_to(&url)?;
        tab.wait_until_navigated()?;
        let title = tab.get_title()?;
        tab.close_with_unload()?;
        Ok(title)
    })
}
