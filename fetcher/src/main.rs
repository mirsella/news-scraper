mod launcher;
mod sources;
use anyhow::Result;
use clap::Parser;
use log::{error, info, trace};
use shared::{config::Config, db_news::DbNews, *};
use std::{
    process::exit,
    sync::{Arc, Mutex},
};
use surrealdb::{engine::remote::ws::Wss, opt::auth::Root, Surreal};

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
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();
    if cli.list {
        println!("Available sources:");
        sources::SOURCES.iter().for_each(|s| println!("{}", s.0));
        return Ok(());
    }

    let mut config = Config::load(&cli.env_file).unwrap_or_else(|e| {
        error!("{}: {}: {}", cli.env_file, e, e.root_cause());
        exit(1);
    });
    if let Some(value) = cli.headless {
        config.chrome_headless = Some(value);
    }

    let telegram = Telegram::new(config.telegram_token.clone(), config.telegram_id);
    let telegram = Arc::new(telegram);

    let db = Surreal::new::<Wss>(&config.surrealdb_host).await?;
    db.signin(Root {
        username: &config.db_user,
        password: &config.db_password,
    })
    .await?;
    db.use_ns("news").use_db("news").await?;

    let seen_urls: Vec<String> = db.query("select link from news").await?.take((0, "link"))?;
    let seen_urls = Arc::new(Mutex::new(seen_urls));
    let mut rx = launcher::init(
        &config,
        cli.enable.unwrap_or_default(),
        seen_urls,
        telegram.clone(),
    );
    let mut counter = 0;
    while let Some(recved) = rx.recv().await {
        let news = match recved {
            Ok(news) => news,
            Err(err) => {
                error!("recv: {:?}", err);
                if let Err(e) = telegram.send(format!("fetcher: recv: {err}")) {
                    error!("telegram.send: {:?}", e);
                }
                continue;
            }
        };
        trace!(
            "recv news: {}: {:.20?}..., link: {:?}",
            news.provider,
            news.title,
            news.link
        );
        let html_body = sanitize_html(&news.body);
        let text_body = extract_clean_text(&html_body);
        let result: Result<Vec<DbNews>, surrealdb::Error> = db
            .create("news")
            .content(DbNews {
                title: news.title.into(),
                link: news.link.into(),
                html_body: html_body.into(),
                text_body: text_body.into(),
                provider: news.provider.into(),
                date: news.date.into(),
                caption: news.caption.into(),
                ..Default::default()
            })
            .await;
        if let Err(e) = result {
            error!("db.create: {:#?}", e);
            continue;
        }
        counter += 1;
    }
    info!("Total news recorded: {}", counter);
    Ok(())
}
