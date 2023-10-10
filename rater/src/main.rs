use anyhow::{anyhow, Result};
use async_openai::{config::OpenAIConfig, Client as ChatClient};
use log::{error, info, trace};
use shared::{config::Config, db_news::DbNews};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use surrealdb::{
    engine::remote::ws::Ws,
    opt::{auth::Root, RecordId},
    Surreal,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        info!("Ctrl-C received!");
        r.store(false, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    let config = Config::load(".env").unwrap_or_else(|e| {
        error!("config: {:?}", e);
        exit(1);
    });

    let db = Surreal::new::<Ws>(&config.surrealdb_host).await?;
    db.signin(Root {
        username: &config.db_user,
        password: &config.db_password,
    })
    .await?;
    db.use_ns("news").use_db("news").await?;

    let openai =
        ChatClient::with_config(OpenAIConfig::default().with_api_key(&config.openai_api_key));

    // TODO: use semaphore to limit parallel rating
    // let sem = Arc::new(tokio::sync::Semaphore::new(10))

    let news: Option<Vec<DbNews>> = db
        .query("select * from news where rating == none AND date > time::floor(time::now(), 1w)")
        .await?
        .take(0)?;

    let res: Result<()> = loop {
        if !running.load(Ordering::Relaxed) {
            break Ok(());
        }
        let mut news = match news {
            Err(e) if e.to_string() == "no news found" => {
                trace!("no news to process");
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }
            Err(e) => break Err(e.context("failed to get news from db")),
            Ok(news) => news,
        };
        let id = news.id.clone().expect("no id wtf");
        trace!("processing {}, {}", id.id, news.link);

        match news.rate(&openai, &config.rating_chat_prompt).await {
            Ok(res) => {
                info!("rating {} ({}): {:?}", id, news.link, res);
            }
            Err(e) => {
                error!("rate: {:?}", e)
            }
        }
        if let Err(e) = news.save(&db).await {
            error!("save: {e}")
        }
    };
    res
}
