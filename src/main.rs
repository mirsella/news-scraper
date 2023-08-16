mod config;
mod newsfetcher;
use log::{error, info};

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut rx = newsfetcher::new();
    while let Some(recved) = rx.recv().await {
        // TODO: implement logging to db
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
