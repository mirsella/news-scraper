use anyhow::Result;
use surrealdb::{
    Connection, Surreal,
    types::{RecordId, SurrealValue},
};

#[derive(SurrealValue)]
struct NewsLink {
    id: RecordId,
    link: String,
}

pub async fn seen_links<C: Connection>(db: &Surreal<C>) -> Result<Vec<String>> {
    let mut links = Vec::new();
    let mut after: Option<RecordId> = None;
    loop {
        // Keyset pagination keeps each response below the WebSocket message limit
        // without repeatedly skipping all earlier records as the database grows.
        let query = if after.is_some() {
            "SELECT id, link FROM news WHERE id > $after ORDER BY id LIMIT 1000"
        } else {
            "SELECT id, link FROM news ORDER BY id LIMIT 1000"
        };
        let page: Vec<NewsLink> = db.query(query).bind(("after", after)).await?.take(0)?;
        let count = page.len();
        after = page.last().map(|item| item.id.clone());
        links.extend(page.into_iter().map(|item| item.link));
        if count < 1000 {
            return Ok(links);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::ensure;
    use surrealdb::{
        engine::remote::ws::Ws,
        opt::{Config, WebsocketConfig, auth::Root},
    };

    #[tokio::test]
    #[ignore = "requires SURREALDB_TEST_HOST, SURREALDB_TEST_USER and SURREALDB_TEST_PASSWORD"]
    async fn seen_links_fit_bounded_websocket_messages() -> Result<()> {
        let host = std::env::var("SURREALDB_TEST_HOST")?;
        let credentials = Root {
            username: std::env::var("SURREALDB_TEST_USER")?,
            password: std::env::var("SURREALDB_TEST_PASSWORD")?,
        };
        let admin = Surreal::new::<Ws>(&host).await?;
        admin.signin(credentials.clone()).await?;
        let namespace = format!(
            "link_pages_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_micros()
        );
        admin.use_ns(&namespace).use_db("news").await?;

        let result: Result<()> = async {
            admin.query("DEFINE TABLE news SCHEMALESS").await?.check()?;
            let config =
                Config::new().websocket(WebsocketConfig::new().max_message_size(512 * 1024))?;
            let reader = Surreal::new::<Ws>((&host, config)).await?;
            reader.signin(credentials).await?;
            reader.use_ns(&namespace).use_db("news").await?;
            ensure!(
                seen_links(&reader).await?.is_empty(),
                "expected an empty database"
            );

            // The full result exceeds the reader's cap, but each 1000-row page fits.
            let records: Vec<NewsLink> = (0..2505)
                .map(|index| NewsLink {
                    id: RecordId::new("news", format!("item_{index:05}")),
                    link: format!("https://example.invalid/{}?id={index}", "a".repeat(320)),
                })
                .collect();
            let expected: Vec<String> = records.iter().map(|item| item.link.clone()).collect();
            let _: Vec<NewsLink> = admin.insert("news").content(records).await?;
            let actual = seen_links(&reader).await?;
            ensure!(
                actual == expected,
                "pagination omitted, duplicated or reordered links"
            );
            Ok(())
        }
        .await;
        admin
            .query("REMOVE NAMESPACE $namespace")
            .bind(("namespace", namespace))
            .await?
            .check()?;
        result
    }
}
