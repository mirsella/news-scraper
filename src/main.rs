mod config;
mod newsfetcher;
use clap::Parser;
use log::{error, info};

#[derive(Parser, Debug)]
struct Cli {
    #[arg(long, short, value_delimiter = ',', num_args = 1..)]
    enabled: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();
    let config = crate::config::load_config(None);
    let mut rx = newsfetcher::new(&config, cli.enabled.unwrap_or_default());
    let mut counter = 0;
    while let Some(recved) = rx.recv().await {
        let news = match recved {
            Ok(news) => news,
            Err(err) => {
                error!("recv: {:#?}", err);
                continue;
            }
        };
        println!("title: {:.40?}..., link: {:?}", news.title, news.link);
        counter += 1;
    }
    info!("Total news: {}", counter)
}
