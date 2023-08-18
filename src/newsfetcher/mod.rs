automod::dir!("src/newsfetcher");

use crate::config::Config;
use futures::{stream::FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::error;
use macros::vec_sources_fn;
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

#[derive(Debug)]
pub struct News {
    pub title: String,
}

pub fn new(config: &Config) -> Receiver<anyhow::Result<News>> {
    let (tx, rx) = channel(100);

    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(config.headless)
            .build()
            .unwrap(),
    )
    .unwrap();

    let mut sources = vec_sources_fn!("src/newsfetcher");
    let mut futures: FuturesUnordered<JoinHandle<anyhow::Result<Vec<News>>>> =
        FuturesUnordered::new();

    for _ in 0..config.concurrent_tabs {
        if let Some(fetch) = sources.pop() {
            let tab = browser.new_tab().unwrap();
            futures.push(spawn_blocking(move || fetch(tab)));
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
            // if let Some(url) = urls.pop() {
            //     futures.push(get_title(browser.clone(), url.to_string()));
            // }
        }
    });
    rx
}

// fn get_title(browser: Browser, url: String) -> JoinHandle<anyhow::Result<String>> {
//     spawn_blocking(move || -> anyhow::Result<String> {
//         let tab = browser.new_tab()?;
//         tab.enable_stealth_mode()?;
//         tab.navigate_to(&url)?;
//         tab.wait_until_navigated()?;
//         let title = tab.get_title()?;
//         tab.close_with_unload()?;
//         Ok(title)
//     })
// }
