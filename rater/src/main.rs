use anyhow::{Context, Result};
use async_openai::{config::OpenAIConfig, Client as ChatClient};
use log::{debug, error, info, trace};
use shared::Telegram;
use shared::{config::Config, db_news::DbNews};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::{engine::remote::ws::Client as WsClient, opt::auth::Root, Surreal};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

async fn retrieve_db_news(db: &Surreal<WsClient>) -> Result<Vec<DbNews>> {
    let db_news: Vec<DbNews> = db
        .query(
            "select * from news
where rating == none
AND date > time::floor(time::now(), 5d)
AND used == false
AND !string::contains(note, 'error rating')
ORDER BY date DESC",
        )
        .await?
        .take(0)?;
    Ok(db_news)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        info!("Ctrl-C received!");
        r.store(false, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    let config = Config::load(".env").unwrap_or_else(|e| {
        error!("config: {}", e);
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
                tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
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
                debug!("processing {}, {}", id.id, news.link);
                let rating = match news.rate(&openai, &rating_chat_prompt).await {
                    Ok(rating) => Some(rating),
                    Err(e) if e.to_string().to_lowercase().contains("bad gateway") => {
                        error!("bad gateway: {:?}", e.to_string());
                        telegram.send("bad gateway !")?;
                        news.rating = None;
                        None
                    }
                    Err(e) => {
                        error!("rating {id}: '{e}'");
                        news.rating = Some(0);
                        news.note = format!("rating failed: {e}").into();
                        telegram.send(format!("rater: rating failed: {e}, {e:?}"))?;
                        None
                    }
                };
                info!("{id} rating: {rating:?}");
                match news.save(&db).await.context("news.save") {
                    Ok(_) => Ok(Some(news)),
                    Err(e) => {
                        error!("saving {id} with {rating:?}: '{e}'");
                        telegram.send(format!("rater: rating failed: {e}, {e:?}"))?;
                        Err(e)
                    }
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            if let Err(e) = handle.await? {
                running.store(false, Ordering::Relaxed);
                error!("handle errored, exitting: {}", e.to_string());
                if let Err(e) = telegram.send(format!("rater: thread error: {}", e)) {
                    error!("TelegramError: {}", e);
                }
            }
            news_done += 1;
            info!("{news_done}/{total_news} done.")
        }
        info!("{total_news} done.");
    }
}
