use std::{error::Error, process::exit};

use log::{error, trace};
use shared::{Config, DbNews};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
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
        let mut db_news: Vec<DbNews> = db
            .query("select * from news where rating == none")
            .await?
            .take(0)?;
        if db_news.is_empty() {
            break Ok(());
        }

        while let Some(news) = db_news.pop() {
            trace!("processing id {:?}, {:?}", news.id, news.link);
            // db.update::<Option<DbNews>>(("news", news.id.unwrap()))
            //     .await?;
        }
        // return Ok(());
    }
}
