use anyhow::Result;
use async_openai::{config::OpenAIConfig, Client as ChatClient};
use env_logger::Builder;
use futures::future::select_all;
use log::{error, info, trace, warn};
use shared::Telegram;
use shared::{config::Config, db_news::DbNews};
use std::env;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::{engine::remote::ws::Client as WsClient, opt::auth::Root, Surreal};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

async fn retrieve_db_news(db: &Surreal<WsClient>) -> Result<Vec<DbNews>> {
    let db_news: Vec<DbNews> = db
        .query(
            "if $PROD=1 { return select *, omit html_body from news
where rating == none
AND date > time::floor(time::now() - 7d, 1d)
AND used == false
AND !string::contains(note, 'error rating')
ORDER BY date DESC }",
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
        .init();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || match r.load(Ordering::Relaxed) {
        true => {
            info!("Ctrl-C received!");
            r.store(false, Ordering::Relaxed);
        }
        false => {
            info!("Ctrl-C received again, exiting!");
            exit(1);
        }
    })
    .expect("Error setting Ctrl-C handler");

    let config = Config::load(".env").unwrap_or_else(|e| {
        error!(".env: {:?}", e);
        exit(1);
    });

    let db = Surreal::new::<Ws>(&config.surrealdb_host).await?;
    db.signin(Root {
        username: &config.db_user,
        password: &config.db_password,
    })
    .await?;
    db.use_ns("news").use_db("news").await?;
    let db = Arc::new(db);

    let openai =
        ChatClient::with_config(OpenAIConfig::default().with_api_key(&config.openai_api_key));
    let openai = Arc::new(openai);

    let telegram = Telegram::new(config.telegram_token.clone(), config.telegram_id);
    let telegram = Arc::new(telegram);
    let sem = Arc::new(Semaphore::new(config.parallel_rating));
    let rating_chat_prompt = Arc::new(config.rating_chat_prompt.clone());

    loop {
        if !running.load(Ordering::Relaxed) {
            return Ok(());
        }
        let total_news;
        let mut news_done = 0;
        let db_news = retrieve_db_news(&db).await;
        let db_news = match db_news {
            Ok(news) if news.is_empty() => {
                trace!("no news to process");
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
            let id = news.id.clone().expect("no id wtf");
            let sem = sem.clone();
            let openai = openai.clone();
            let db = db.clone();
            let rating_chat_prompt = rating_chat_prompt.clone();
            let running = running.clone();
            let telegram = telegram.clone();
            let handle: JoinHandle<Result<Option<DbNews>>> = tokio::spawn(async move {
                let _permit = sem.acquire().await;
                if !running.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                trace!("processing {}, {}", id.id, news.link);
                let rating = match news.rate(&openai, rating_chat_prompt.as_ref()).await {
                    Ok(rating) => Some(rating),
                    Err(e) if e.to_string().to_lowercase().contains("bad gateway") => {
                        error!("bad gateway: {:?}", e.to_string());
                        dbg!(&e);
                        news.rating = None;
                        None
                    }
                    Err(e) if e.to_string().to_lowercase().contains("service unavailable") => {
                        error!("service unavailable: {:?}", e.to_string());
                        dbg!(&e);
                        news.rating = None;
                        None
                    }
                    Err(e) => {
                        error!("rating {id}: {e}");
                        news.rating = Some(0);
                        let newline = if news.note.is_empty() { "" } else { "\n" };
                        news.note = format!("{}{newline}rating failed: '{e}'", news.note).into();
                        // only send telegram notif if it's not a "sorry" message
                        if !e.to_string().to_lowercase().contains("i'm sorry") {
                            telegram
                                .send(format!("rater: {id} {} rating failed: {e}", news.link))?;
                        }
                        None
                    }
                };
                info!("{id} rating: {rating:?}");
                match news.save(&db).await {
                    Ok(_) => Ok(Some(news)),
                    Err(e) => {
                        warn!("saving {id} failed one time: {e:#?}");
                        // telegram.send(format!("rater: saving {id} failed one time: {e:#?}"))?;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        let e = match news.save(&db).await {
                            Ok(_) => return Ok(Some(news)),
                            Err(e) => e,
                        };
                        error!("saving {id} with {rating:?} second time: {e:#?}");
                        telegram.send(format!("rater: re-saving {id} failed: {e:#?}"))?;
                        Err(e)
                    }
                }
            });
            handles.push(handle);
        }
        while !handles.is_empty() {
            let (handle, _index, remaining) = select_all(handles).await;
            handles = remaining;
            match handle? {
                Err(e) => {
                    running.store(false, Ordering::Relaxed);
                    if let Err(e) = telegram.send(format!("rater: thread error: {}", e)) {
                        error!("TelegramError: {}", e);
                    }
                    return Err(e);
                }
                Ok(Some(news)) => {
                    news_done += 1;
                    info!("{} {news_done}/{total_news} done.", news.id.expect("no id"))
                }
                _ => (),
            }
        }
        info!("finished this batch {news_done}/{total_news}.")
    }
}
