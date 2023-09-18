mod newsfetcher;
mod sources;
use std::{
    error::Error,
    process::exit,
    sync::{Arc, Mutex},
};

use clap::Parser;
use log::{debug, error, info};
use shared::*;
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};

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
    #[arg(long, default_value = ".env")]
    env_file: String,
    #[arg(long, help = "Run chrome in headless mode")]
    headless: Option<bool>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();
    if cli.list {
        println!("Available sources:");
        sources::SOURCES.iter().for_each(|s| println!("{}", s.0));
        return Ok(());
    }

    let mut config = Config::load(&cli.env_file).unwrap_or_else(|e| {
        error!("{}: {}", cli.env_file, e);
        exit(1);
    });
    if let Some(value) = cli.headless {
        config.chrome_headless = Some(value);
    }
    println!("config: {:?}", config);

    let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;
    db.signin(Root {
        username: &config.db_user,
        password: &config.db_password,
    })
    .await?;
    db.use_ns("news").use_db("news").await?;

    let seen_urls: Vec<String> = db.query("select link from news").await?.take((0, "link"))?;
    let seen_urls = Arc::new(Mutex::new(seen_urls));
    let mut rx = newsfetcher::new(&config, cli.enable.unwrap_or_default(), seen_urls);
    let mut counter = 0;
    while let Some(recved) = rx.recv().await {
        let news = match recved {
            Ok(news) => news,
            Err(err) => {
                error!("recv: {:#?}", err);
                continue;
            }
        };
        debug!(
            "recv news: title: {:.20?}..., link: {:?}",
            news.title, news.link
        );
        let result: Result<Vec<DbNews>, surrealdb::Error> = db
            .create("news")
            .content(DbNews {
                title: news.title.into(),
                link: news.link.into(),
                provider: news.provider.into(),
                date: news.date.into(),
                body: news.body.into(),
                caption: news.caption.into(),
                ..Default::default()
            })
            .await;
        if result.is_err() {
            error!("db.create: {:#?}", result);
            continue;
        }
        counter += 1;
    }
    info!("Total news: {}", counter);
    Ok(())
}
