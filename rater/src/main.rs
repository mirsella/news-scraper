use std::process::exit;

use anyhow::{anyhow, Result};
use log::{error, trace};
use nanohtml2text::html2text;
use shared::{Config, DbNews};
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    sql::Thing,
    Surreal,
};

fn clean_string(s: &str) -> String {
    s.split_whitespace()
        .map(|s| s.trim().replace('\n', ""))
        .collect::<Vec<String>>()
        .join(" ")
}

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

    loop {
        let news: Option<DbNews> = db
            .query(
                "select * from only news where rating == none AND date > time::floor(time::now(), 1w) AND locked == false limit 1",
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
        let id = news.id.unwrap();
        if let Err(e) = lock_news(&db, &id).await {
            error!("failed to lock news {id}: {e}");
        };
        trace!(
            "processing id {} body size {}, {}",
            id.id,
            news.body.len(),
            news.link
        );
        let text = html2text(&news.body);
        let text = clean_string(&text);

        // TODO: actually rate the news
        let rating: Option<i64> = None;
        db.update::<Option<DbNews>>(("news", id))
            .merge(serde_json::json!({"rating": rating, "locked": false }))
            .await?;
    }
}
