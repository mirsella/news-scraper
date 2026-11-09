use anyhow::{Context, Result};
use async_openai::{Client as ChatClient, config::OpenAIConfig};
use env_logger::Builder;
use futures::future::select_all;
use log::{error, info, trace, warn};
use shared::Telegram;
use shared::{config::Config, db_news::DbNews};
use std::env;
use std::io::Write;
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use surrealdb::engine::remote::http::Http;
use surrealdb::{Surreal, opt::auth::Root, types::ToSql};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

async fn retrieve_db_news<T: surrealdb::Connection>(db: &Surreal<T>) -> Result<Vec<DbNews>> {
    let db_news: Vec<DbNews> = db
        .query(
            "return select * from news
where rating == none
AND date >= time::now() - 1w
AND !string::contains(note, 'error rating')
ORDER BY date DESC limit 500",
        )
        .await?
        .take(0)?;
    Ok(db_news)
}

fn sleep_check(running: &AtomicBool, duration: Duration) {
    let mut slept = Duration::from_secs(0);
    while slept < duration {
        if !running.load(Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
        slept += Duration::from_secs(1);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    Builder::new()
        .parse_filters(&env::var("RUST_LOG").unwrap_or("rater=trace".into()))
        .format(|buf, record| {
            shared::telemetry::log(record);
            writeln!(
                buf,
                "[{} {} {}] {}",
                buf.timestamp(),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || match r.load(Ordering::Relaxed) {
        true => {
            info!("Shutdown signal received");
            r.store(false, Ordering::Relaxed);
        }
        false => {
            info!("Second shutdown signal received, exiting");
            shared::telemetry::flush();
            exit(1);
        }
    })
    .expect("Error setting Ctrl-C handler");

    let config = Config::load(".env").unwrap_or_else(|e| {
        error!(".env: {:?}", e);
        exit(1);
    });
    anyhow::ensure!(
        !matches!(
            config.rating_reasoning_effort,
            async_openai::types::chat::ReasoningEffort::Minimal
                | async_openai::types::chat::ReasoningEffort::Max
        ),
        "unsupported RATING_REASONING_EFFORT (expected none/low/medium/high/xhigh)"
    );
    anyhow::ensure!(
        config.rating_max_completion_tokens > 0,
        "RATING_MAX_COMPLETION_TOKENS must be positive"
    );
    let _telemetry = shared::telemetry::start("rater");
    info!(
        "rating model={} effort={:?} output_limit={}",
        config.rating_model, config.rating_reasoning_effort, config.rating_max_completion_tokens
    );
    let config = Arc::new(config);

    let db = Surreal::new::<Http>(&config.surrealdb_host).await?;
    db.signin(Root {
        username: config.db_user.clone(),
        password: config.db_password.clone(),
    })
    .await
    .context("connecting to surrealdb")?;
    db.use_ns("news").use_db("news").await?;
    let db = Arc::new(db);

    let openai =
        ChatClient::with_config(OpenAIConfig::default().with_api_key(&config.openai_api_key));
    let openai = Arc::new(openai);

    let telegram = Telegram::new(config.telegram_token.clone(), config.telegram_id);
    let telegram = Arc::new(telegram);
    let sem = Arc::new(Semaphore::new(config.parallel_rating));
    let prompt = include_str!("../../rating-prompt.md");

    loop {
        if !running.load(Ordering::Relaxed) {
            return Ok(());
        }
        let total_news;
        let mut news_done = 0;
        let db_news = retrieve_db_news(&db).await;
        let db_news = match db_news {
            Ok(news) if news.is_empty() => {
                info!("no news to process");
                sleep_check(&running, Duration::from_secs(60));
                continue;
            }
            Ok(news) => {
                total_news = news.len();
                info!("got {} news to process", total_news);
                news
            }
            Err(e) => return Err(e.context("failed to get news from db")),
        };

        let mut handles = Vec::with_capacity(db_news.len());

        for mut news in db_news {
            let id = news
                .id
                .as_ref()
                .context("news record is missing its ID")?
                .to_sql();
            let sem = sem.clone();
            let openai = openai.clone();
            let db = db.clone();
            let running = running.clone();
            let telegram = telegram.clone();
            let config = config.clone();
            let handle: JoinHandle<Result<Option<DbNews>>> = tokio::spawn(async move {
                let permit = sem.acquire().await;
                if !running.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                trace!("processing {id}, {}", news.link);
                let rated = match news.rate(&openai, prompt, &config).await {
                    Ok(()) => true,
                    Err(e) if e.to_string().to_lowercase().contains("bad gateway") => {
                        error!("bad gateway: {:?}", e.to_string());
                        dbg!(&e);
                        news.rating = None;
                        false
                    }
                    Err(e) if e.to_string().to_lowercase().contains("service unavailable") => {
                        error!("service unavailable: {:?}", e.to_string());
                        dbg!(&e);
                        news.rating = None;
                        false
                    }
                    Err(e) => {
                        error!("rating {id}: {e:?}");
                        news.rating = Some(0);
                        let newline = if news.note.is_empty() { "" } else { "\n" };
                        news.note = format!("{}{newline}rating failed: '{e}'", news.note).into();
                        // only send telegram notif if it's not a "sorry" message
                        if !e.to_string().to_lowercase().contains("i'm sorry") {
                            telegram
                                .send(format!("rater: {id} {} rating failed: {e:?}", news.link))?;
                        }
                        false
                    }
                };
                info!(
                    "{id} positivity: {:?}, travel: {:?}",
                    news.rating, news.rating_travel
                );
                shared::telemetry::capture(
                    "article_rating_completed",
                    serde_json::json!({
                        "success":rated, "provider":news.provider,
                        "model":config.rating_model, "reasoning_effort":config.rating_reasoning_effort,
                        "positivity_rating":news.rating, "travel_rating":news.rating_travel,
                    }),
                );
                let save_result = match news.save(&db).await {
                    Ok(_) => Ok(Some(news)),
                    Err(e) => {
                        warn!("saving {id} failed one time: {e:#?}");
                        // telegram.send(format!("rater: saving {id} failed one time: {e:#?}"))?;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        match news.save(&db).await {
                            Ok(_) => Ok(Some(news)),
                            Err(e) => {
                                error!("saving {id} second time: {e:#?}");
                                telegram.send(format!("rater: re-saving {id} failed: {e:#?}"))?;
                                Err(e)
                            }
                        }
                    }
                };
                drop(permit);
                save_result
            });
            handles.push(handle);
        }
        while !handles.is_empty() {
            let (handle, _index, remaining) = select_all(handles).await;
            handles = remaining;
            match handle? {
                Err(e) => {
                    running.store(false, Ordering::Relaxed);
                    if let Err(e) = telegram.send(format!("rater: thread error: {e}")) {
                        error!("TelegramError: {}", e);
                    }
                    return Err(e);
                }
                Ok(Some(news)) => {
                    news_done += 1;
                    info!(
                        "{} {news_done}/{total_news} done.",
                        news.id.expect("no id").to_sql()
                    );
                }
                _ => (),
            }
        }
        info!("finished this batch {news_done}/{total_news}.");
    }
}
