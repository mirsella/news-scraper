use std::{process::exit, time::Duration};

use anyhow::{anyhow, Result};
use async_openai::Client as ChatClient;
use log::{error, trace};
use shared::{Config, DbNews};
use surrealdb::{
    engine::remote::ws::{Client as DbClient, Ws},
    opt::{auth::Root, RecordId},
    sql::Thing,
    Surreal,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = Config::load(".env").unwrap_or_else(|e| {
        error!(".env: {}", e);
        exit(1);
    });

    let db = Surreal::new::<Ws>(&config.surrealdb_host).await?;
    db.signin(Root {
        username: &config.db_user,
        password: &config.db_password,
    })
    .await?;
    db.use_ns("news").use_db("news").await?;

    let client = ChatClient::new();

    let mut last_id: Option<RecordId> = None;
    let res: Result<()> = loop {
        let news = DbNews::get_nonrated(&db).await;
        let news = match news {
            None => {
                trace!("no news to process");
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }
            Some(news) => news,
        };
        if news.id == last_id {
            break Err(anyhow!("found two time the same unrated news"));
        }
        let id = news.id.clone().unwrap();
        if let Err(e) = lock_news(&db, &id).await {
            error!("failed to lock news {id}: {e}");
        };
        trace!(
            "processing id {}, text size {}, {}",
            id.id,
            news.text_body.len(),
            news.link
        );

        // TODO: actually rate the news
        std::thread::sleep(Duration::from_secs(1));

        let rating: Option<i64> = None;
        db.update::<Option<DbNews>>(("news", id))
            .merge(serde_json::json!({"rating": rating, "locked": false }))
            .await?;
        last_id = news.id;
    };
    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}
