use std::{process::exit, time::Duration};

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

fn extract_clean_text(html: &str) -> String {
    let s = html2text(html);
    let re = regex::Regex::new(r"\(?https?://[^\s]+").unwrap();
    let s = re.replace_all(&s, "").to_string();
    let s = s
        .split_whitespace()
        .map(|s| s.trim().replace('\n', ""))
        .collect::<Vec<String>>()
        .join(" ");
    s
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
        let text = extract_clean_text(&news.body);

        println!("{}", text);

        // TODO: actually rate the news
        std::thread::sleep(Duration::from_secs(1));

        let rating: Option<i64> = None;
        db.update::<Option<DbNews>>(("news", id))
            .merge(serde_json::json!({"rating": rating, "locked": false }))
            .await?;
    }
}
