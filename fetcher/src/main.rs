mod launcher;
mod sources;
use anyhow::Result;
use chrono::DateTime;
use clap::builder::PossibleValuesParser;
use clap::Parser;
use env_logger::Builder;
use log::{debug, error, info, trace};
use shared::{config::Config, db_news::DbNews, extract_clean_text, sanitize_html, Telegram};
use sources::{extract_prefix_from_provider, SeenLink, SOURCES};
use std::{
    borrow::Cow,
    convert::Into,
    env,
    process::{self, exit},
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, RwLock,
    },
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
    #[arg(long, short, value_delimiter = ',', num_args = 1.., help = "Enable ONLY specified news sources", value_parser = PossibleValuesParser::new(SOURCES.iter().map(|(name, _)| *name)))]
    enable: Option<Vec<String>>,
    #[arg(long, default_value = ".env")]
    env_file: String,
    #[arg(long, help = "Run chrome in headless mode")]
    headless: Option<bool>,
    #[arg(
        long,
        help = "Don't throw an error if the database is empty",
        default_value = "false"
    )]
    ignore_empty_db: bool,
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
        error!("{}: {:?}", cli.env_file, e);
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

    let counter = Arc::new(AtomicU16::default());
    {
        let counter = counter.clone();
        ctrlc::set_handler(move || {
            info!("ctrl-c received, exiting. ");
            info!("Total news recorded: {}", counter.load(Ordering::Relaxed));
            process::exit(0);
        })
        .unwrap();
    }
    {
        let counter = counter.clone();
        std::panic::set_hook(Box::new(move |panic_info| {
            println!(
                "Exiting due to panic. Number of tasks done: {}",
                counter.load(Ordering::SeqCst)
            );
            println!("Panic info: {panic_info}");
            process::exit(1);
        }));
    }

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

    assert!(!seen_news.is_empty() || cli.ignore_empty_db);
    info!("Total news already seen: {}", seen_news.len());

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
        news.tags.push(extract_prefix_from_provider(&news.provider));
        let error: Option<anyhow::Error>;
        if seen_news
            .read()
            .unwrap()
            .iter()
            .any(|seen_link| seen_link.link == news.link)
        {
            debug!(
                "news already seen with different provider, merging: tags: {:?}, link: {}",
                news.tags, news.link
            );
            let result: Result<_, surrealdb::Error> = db
                .query("update news set tags = array::union(tags, $newtags) where link = $link return none")
                .bind(("newtags", news.tags.clone()))
                .bind(("link", news.link.clone()))
                .await;
            error = match result {
                Ok(result) => result.check().err().map(Into::into),
                Err(e) => Some(e.into()),
            };
        } else {
            debug!("new news: {}", news.link);
            let html_body = sanitize_html(&news.body);
            let text_body = extract_clean_text(&html_body);
            let result: Result<Vec<DbNews>, surrealdb::Error> = db
                .create("news")
                .content(DbNews {
                    title: news.title.into(),
                    link: Cow::Owned(news.link.clone()),
                    tags: news.tags.clone(),
                    html_body: html_body.into(),
                    text_body: text_body.into(),
                    provider: news.provider.into(),
                    date: DateTime::from(news.date).into(),
                    caption: news.caption.into(),
                    ..Default::default()
                })
                .await;
            error = result.err().map(Into::into);
        }
        seen_news.write().unwrap().push(SeenLink {
            link: news.link,
            tags: news.tags,
        });
        if let Some(e) = error {
            error!("db: {e:#?}");
            telegram.send(format!("fetcher: db: {e:#?}")).ok();
            thread::sleep(Duration::from_secs(1));
            continue;
        } else {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }
    info!("Total news recorded: {}", counter.load(Ordering::Relaxed));
    Ok(())
}
