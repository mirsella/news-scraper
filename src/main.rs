mod config;
mod newsfetcher;
use log::{error, info};

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = crate::config::load_config(None);
    let mut rx = newsfetcher::new(&config);
    while let Some(recved) = rx.recv().await {
        let news = match recved {
            Ok(news) => news,
            Err(err) => {
                error!("recv: {:#?}", err);
                continue;
            }
        };
        info!("{news:?}");
    }
}
