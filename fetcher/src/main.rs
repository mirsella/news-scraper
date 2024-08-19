mod launcher;
mod sources;
use anyhow::Result;
use chrono::DateTime;
use clap::Parser;
use env_logger::Builder;
use log::{error, info, trace};
use shared::{config::Config, db_news::DbNews, *};
use sources::{SeenLink, SOURCES};
use std::{
    env,
    process::exit,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
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
async fn main() -> Result<()> {
    Builder::new()
        .parse_filters(&env::var("RUST_LOG").unwrap_or("fetcher=debug".into()))
        .init();

    let cli = Cli::parse();
    if cli.list {
        println!("Available sources:");
        for (name, _) in sources::SOURCES {
            println!("{name}");
        }
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

    let db = Surreal::new::<Ws>(&config.surrealdb_host).await?;
    db.signin(Root {
        username: &config.db_user,
        password: &config.db_password,
    })
    .await?;
    db.use_ns("news").use_db("news").await?;

    let mut counter = 0;
    let sources: Vec<_> = match cli.enable {
        Some(ref enabled) => SOURCES
            .iter()
            .filter(|s| enabled.contains(&s.0.to_string()))
            .collect(),
        None => SOURCES.iter().collect(),
    };
    // provider, link
    let seen_news: Vec<SeenLink> = db
        .query("select link, tags from news")
        .await?
        .take(0)
        .unwrap_or_default();
    let seen_news = Arc::new(RwLock::new(seen_news));
    let mut rx = launcher::init(&config, sources, seen_news.clone(), telegram.clone());
    while let Some(recved) = rx.recv().await {
        let mut news = match recved {
            Ok(news) => news,
            Err(err) => {
                error!("recv: {:#?}", err);
                if let Err(e) = telegram.send(format!("fetcher: recv: {err:#?}")) {
                    error!("telegram.send: {:#?}", e);
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
        if seen_news
            .read()
            .unwrap()
            .iter()
            .any(|SeenLink(l, _)| l == &news.link)
        {
            log::warn!(
                "news already seen with different provider, merging: tags: {:?}, link: {}",
                news.tags,
                news.link
            );
            // TODO: add the prefix to the tags: fr, lme, ci, etc...
            let result: Result<_, surrealdb::Error> = db
                .query("update news set tags = array::union(tags, $newtags) return none")
                .bind(("newtags", news.tags))
                .await;
            if let Err(e) = result {
                error!("db merge new tags: {:#?}", e);
                thread::sleep(Duration::from_secs(5));
                continue;
            }
            exit(0);
        } else {
            news.tags.push(
                news.provider
                    .split_once("::")
                    .expect("a valid provider with ::")
                    .0
                    .to_string(),
            );
            let html_body = sanitize_html(&news.body);
            let text_body = extract_clean_text(&html_body);
            let result: Result<Vec<DbNews>, surrealdb::Error> = db
                .create("news")
                .content(DbNews {
                    title: news.title.into(),
                    link: news.link.into(),
                    tags: news.tags,
                    html_body: html_body.into(),
                    text_body: text_body.into(),
                    provider: news.provider.into(),
                    date: DateTime::from(news.date).into(),
                    caption: news.caption.into(),
                    ..Default::default()
                })
                .await;
            if let Err(e) = result {
                error!("db.create: {:#?}", e);
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        }
        counter += 1;
    }
    info!("Total news recorded: {}", counter);
    Ok(())
}
