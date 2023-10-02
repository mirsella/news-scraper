use std::{process::exit, time::Duration};

use anyhow::{anyhow, Result};
use log::{error, trace};
use shared::{Config, DbNews};
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::{auth::Root, RecordId},
    sql::Thing,
    Surreal,
};

async fn lock_news(db: &Surreal<Client>, id: &Thing) -> Result<()> {
    match db
        .update::<Option<DbNews>>(("news", id.clone()))
        .merge(serde_json::json!({"locked": true }))
        .await
    {
        Ok(Some(_)) => Ok(()),
        Err(e) => Err(anyhow!("{}", e)),
        _ => Err(anyhow!("no news found")),
    }
}

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

    let mut last_id: Option<RecordId> = None;
    let res: Result<()> = loop {
        let news: Option<DbNews> = db
            .query(
                "select * from news where rating == none AND date > time::floor(time::now(), 1w) AND locked == false limit 1",
            )
            .await?
            .take(0)?;
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
