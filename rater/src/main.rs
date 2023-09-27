use std::{error::Error, process::exit};

use log::{error, trace};
use nanohtml2text::html2text;
use shared::{Config, DbNews};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};

fn clean_string(s: &str) -> String {
    s.split_whitespace()
        .map(|s| s.trim().replace('\n', ""))
        .collect::<Vec<String>>()
        .join(" ")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
        match db
            .update::<Option<DbNews>>(("news", id.clone()))
            .merge(serde_json::json!({"locked": true }))
            .await
        {
            Ok(_) => (),
            Err(e) => {
                error!("failed to lock news {}: {}", id.id, e);
                continue;
            }
        }
        trace!(
            "processing id {} body size {}, {}",
            id.id,
            news.body.len(),
            news.link
        );
        let text = html2text(&news.body);
        let text = clean_string(&text);
        let title = clean_string(&news.title);

        // TODO: actually rate the news
        let rating: Option<i64> = None;
        db.update::<Option<DbNews>>(("news", id))
            .merge(serde_json::json!({"rating": rating, "locked": false }))
            .await?;
    }
}
