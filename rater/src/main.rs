mod rating_cache;

use anyhow::{Context, Result};
use async_openai::config::OpenAIConfig;
use env_logger::Builder;
use log::{error, info, trace, warn};
use rating_cache::RatingCache;
use shared::Telegram;
use shared::{
    config::Config,
    db_news::{DbNews, is_temporary_rating_error, rating_client},
};
use std::env;
use std::io::Write;
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::{Surreal, opt::auth::Root, types::ToSql};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

enum RatingOutcome {
    Saved(String),
    Deferred,
    Shutdown,
}

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

async fn wait_before_next_pass(running: &AtomicBool) {
    for _ in 0..60 {
        if !running.load(Ordering::Relaxed) {
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
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
        "rating model={} effort={:?} tier={:?} reuse_duplicates={} output_limit={}",
        config.rating_model,
        config.rating_reasoning_effort,
        config.rating_service_tier,
        config.rating_reuse_exact_duplicates,
        config.rating_max_completion_tokens
    );
    let config = Arc::new(config);

    let db = Surreal::new::<Ws>(&config.surrealdb_host).await?;
    db.signin(Root {
        username: config.db_user.clone(),
        password: config.db_password.clone(),
    })
    .await
    .context("connecting to surrealdb")?;
    db.use_ns("news").use_db("news").await?;
    info!("connected to surrealdb over WebSocket");
    let db = Arc::new(db);

    let openai = rating_client(
        OpenAIConfig::default().with_api_key(&config.openai_api_key),
        config.rating_service_tier,
    );
    let openai = Arc::new(openai);

    let telegram = Telegram::new(config.telegram_token.clone(), config.telegram_id);
    let telegram = Arc::new(telegram);
    let sem = Arc::new(Semaphore::new(config.parallel_rating));
    let prompt = include_str!("../../rating-prompt.md");
    let rating_cache = config
        .rating_reuse_exact_duplicates
        .then(|| Arc::new(RatingCache::default()));

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
                wait_before_next_pass(&running).await;
                continue;
            }
            Ok(news) => {
                total_news = news.len();
                info!("got {} news to process", total_news);
                news
            }
            Err(e) => return Err(e.context("failed to get news from db")),
        };

        let mut tasks = JoinSet::new();

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
            let rating_cache = rating_cache.clone();
            tasks.spawn(async move {
                let _permit = sem.acquire().await?;
                if !running.load(Ordering::Relaxed) {
                    return Ok(RatingOutcome::Shutdown);
                }
                trace!("processing {id}, {}", news.link);
                let result = if let Some(cache) = rating_cache {
                    cache
                        .rate(&mut news, async |article| {
                            article.request_rating(&openai, prompt, &config).await
                        })
                        .await
                } else {
                    news.request_rating(&openai, prompt, &config)
                        .await
                        .map(|rating| {
                            news.apply_rating(&rating);
                            false
                        })
                };
                let (rated, reused) = match result {
                    Ok(reused) => (true, reused),
                    Err(e) if is_temporary_rating_error(&e) => {
                        warn!("rating {id} temporarily unavailable: {e:#}");
                        shared::telemetry::capture(
                            "article_rating_deferred",
                            serde_json::json!({
                                "provider":news.provider, "model":config.rating_model,
                                "reasoning_effort":config.rating_reasoning_effort,
                                "service_tier":config.rating_service_tier,
                            }),
                        );
                        return Ok(RatingOutcome::Deferred);
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
                        (false, false)
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
                        "rating_reused":reused,
                    }),
                );
                if let Err(e) = news.save(&db).await {
                    warn!("saving {id} failed one time: {e:#?}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    if let Err(e) = news.save(&db).await {
                        error!("saving {id} second time: {e:#?}");
                        telegram.send(format!("rater: re-saving {id} failed: {e:#?}"))?;
                        return Err(e);
                    }
                }
                Ok(RatingOutcome::Saved(id))
            });
        }
        let mut retry_next_pass = false;
        while let Some(handle) = tasks.join_next().await {
            match handle? {
                Err(e) => {
                    running.store(false, Ordering::Relaxed);
                    if let Err(e) = telegram.send(format!("rater: thread error: {e}")) {
                        error!("TelegramError: {}", e);
                    }
                    return Err(e);
                }
                Ok(RatingOutcome::Saved(id)) => {
                    news_done += 1;
                    info!("{id} {news_done}/{total_news} done.");
                }
                Ok(RatingOutcome::Deferred) => retry_next_pass = true,
                Ok(RatingOutcome::Shutdown) => (),
            }
        }
        info!("finished this batch {news_done}/{total_news}.");
        if retry_next_pass {
            wait_before_next_pass(&running).await;
        }
    }
}
