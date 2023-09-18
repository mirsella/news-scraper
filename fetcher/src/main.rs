mod newsfetcher;
mod sources;
use std::process::exit;

use clap::Parser;
use log::{error, info};
use shared::*;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(
        long,
        short,
        default_value = "false",
        help = "List available news sources that can be used with --enable"
    )]
    list: bool,
    #[arg(long, short, value_delimiter = ',', num_args = 1.., help = "Enable ONLY specified news sources")]
    enable: Option<Vec<String>>,
    #[arg(long, default_value = "./.env")]
    env_file: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();
    if cli.list {
        println!("Available sources:");
        sources::SOURCES.iter().for_each(|s| println!("{}", s.0));
        return;
    }
    let config = Config::load(&cli.env_file).unwrap_or_else(|e| {
        error!("{}: {}", cli.env_file, e);
        exit(1);
    });
    println!("config: {:?}", config);
    let mut rx = newsfetcher::new(&config, cli.enable.unwrap_or_default());
    let mut counter = 0;
    while let Some(recved) = rx.recv().await {
        let news = match recved {
            Ok(news) => news,
            Err(err) => {
                error!("recv: {:#?}", err);
                continue;
            }
        };
        info!(
            "recv news: title: {:.20?}..., link: {:?}",
            news.title, news.link
        );
        counter += 1;
    }
    info!("Total news: {}", counter)
}
