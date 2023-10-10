use std::process::exit;

use anyhow::{anyhow, Result};
use async_openai::{config::OpenAIConfig, Client as ChatClient};
use log::{error, info, trace};
use shared::{config::Config, db_news::DbNews};
use surrealdb::{
    engine::remote::ws::Ws,
    opt::{auth::Root, RecordId},
    Surreal,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
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

    let openai =
        ChatClient::with_config(OpenAIConfig::default().with_api_key(&config.openai_api_key));

    let mut last_id: Option<RecordId> = None;
    let res: Result<()> = loop {
        let news = DbNews::new_nonrated(&db).await;
        let mut news = match news {
            Err(e) if e.to_string() == "no news found" => {
                trace!("no news to process");
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }
            Err(e) => break Err(e.context("failed to get news from db")),
            Ok(news) => news,
        };
        if news.id == last_id {
            break Err(anyhow!("found two time the same unrated news"));
        }
        let id = news.id.clone().expect("no id wtf");
        trace!(
            "processing id {}, text size {}, {}",
            id.id,
            news.text_body.len(),
            news.link
        );

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
        last_id = news.id;
    };
    res
}
