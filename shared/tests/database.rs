use anyhow::{Context, Result, ensure};
use shared::DbNews;
use surrealdb::{
    Surreal,
    engine::remote::{http::Http, ws::Ws},
    opt::auth::{Namespace, Record, Root},
    types::{RecordId, object},
};

/// Run against a disposable SurrealDB 4.x server, never an application database.
#[tokio::test]
#[ignore = "requires SURREALDB_TEST_HOST, SURREALDB_TEST_USER and SURREALDB_TEST_PASSWORD"]
async fn database_round_trip_and_record_access() -> Result<()> {
    let host = std::env::var("SURREALDB_TEST_HOST")?;
    let db = Surreal::new::<Ws>(&host).await?;
    db.signin(Root {
        username: std::env::var("SURREALDB_TEST_USER")?,
        password: std::env::var("SURREALDB_TEST_PASSWORD")?,
    })
    .await?;
    let namespace = format!(
        "deps_test_{}_{}",
        std::process::id(),
        chrono::Utc::now().timestamp_micros()
    );
    db.use_ns(&namespace).use_db("news").await?;

    // Always clean up this test's namespace, including after a failed assertion/query.
    let result: Result<()> = async {
        db.query(include_str!("../../schema.surql"))
            .await?
            .check()?;
        let article = DbNews {
            title: "A positive test story".into(),
            link: "https://example.com/dependency-test".into(),
            date: chrono::Utc::now().into(),
            provider: "fr::test".into(),
            html_body: "<p>Test article</p>".into(),
            text_body: "Test article".into(),
            tags: vec!["fr".into()],
            ..Default::default()
        };
        let mut saved: DbNews = db
            .create("news")
            .content(article.clone())
            .await?
            .context("CREATE returned no article")?;
        let id = saved.id.clone().context("CREATE returned no record ID")?;
        ensure!(saved.date == article.date, "datetime did not round-trip");
        ensure!(saved.rating.is_none(), "new article must be unrated");

        let pending: Vec<DbNews> = db
            .query(
                "SELECT * FROM news WHERE rating == NONE AND date >= time::now() - 1w \
             AND !string::contains(note, 'error rating') ORDER BY date DESC LIMIT 500",
            )
            .await?
            .take(0)?;
        ensure!(
            pending.len() == 1,
            "rater query did not return the new article"
        );
        saved.rating = Some(80);
        saved.rating_travel = Some(35);
        saved.tags.push("test".into());
        let saved = saved.save(&db).await?;
        ensure!(saved.id == Some(id.clone()), "save changed the record ID");
        ensure!(saved.rating == Some(80), "rating was not saved");
        ensure!(saved.tags == ["fr", "test"], "tags did not round-trip");
        let duplicate: surrealdb::Result<Option<DbNews>> = db.create("news").content(article).await;
        ensure!(duplicate.is_err(), "unique link index allowed a duplicate");

        let reader = Surreal::new::<Ws>(&host).await?;
        reader
            .signup(Record {
                namespace: namespace.clone(),
                database: "news".into(),
                access: "user".into(),
                params: object! { name: "reader", password: "test-password" },
            })
            .await?;
        reader.use_ns(&namespace).use_db("news").await?;
        let hidden: Vec<DbNews> = reader.select("news").await?;
        ensure!(hidden.is_empty(), "inactive user can read articles");

        db.query("UPDATE user SET activated = true WHERE name = 'reader'")
            .await?
            .check()?;
        reader.invalidate().await?;
        reader
            .signin(Record {
                namespace: namespace.clone(),
                database: "news".into(),
                access: "user".into(),
                params: object! { name: "reader", password: "test-password" },
            })
            .await?;
        let visible: Vec<DbNews> = reader.select("news").await?;
        ensure!(visible.len() == 1, "active user cannot read articles");
        reader.invalidate().await?;
        ensure!(
            reader
                .signin(Record {
                    namespace: namespace.clone(),
                    database: "news".into(),
                    access: "user".into(),
                    params: object! { name: "reader", password: "wrong-password" },
                })
                .await
                .is_err(),
            "signin accepted an incorrect password"
        );

        let cleaned: Vec<RecordId> = db
            .query("UPDATE news SET html_body = '', text_body = '' RETURN id")
            .await?
            .take((0, "id"))?;
        ensure!(
            cleaned == [id],
            "cleanup query did not return typed record IDs"
        );
        Ok(())
    }
    .await;
    db.query("REMOVE NAMESPACE $namespace")
        .bind(("namespace", namespace))
        .await?
        .check()?;
    result
}

/// Reproduce the hourly HTTP failure in seconds and exercise the workers' transport.
#[tokio::test]
#[ignore = "requires SURREALDB_TEST_HOST, SURREALDB_TEST_USER and SURREALDB_TEST_PASSWORD"]
async fn websocket_session_outlives_signin_token() -> Result<()> {
    let host = std::env::var("SURREALDB_TEST_HOST")?;
    let admin = Surreal::new::<Ws>(&host).await?;
    admin
        .signin(Root {
            username: std::env::var("SURREALDB_TEST_USER")?,
            password: std::env::var("SURREALDB_TEST_PASSWORD")?,
        })
        .await?;
    let namespace = format!(
        "auth_expiry_{}_{}",
        std::process::id(),
        chrono::Utc::now().timestamp_micros()
    );
    admin.use_ns(&namespace).use_db("news").await?;

    let result: Result<()> = async {
        admin
            .query(
                "DEFINE TABLE auth_probe SCHEMALESS; \
                 DEFINE USER worker ON NAMESPACE PASSWORD 'test-password' \
                 ROLES EDITOR DURATION FOR TOKEN 3s;",
            )
            .await?
            .check()?;
        let credentials = Namespace {
            namespace: namespace.clone(),
            username: "worker".into(),
            password: "test-password".into(),
        };
        let websocket = Surreal::new::<Ws>(&host).await?;
        websocket.signin(credentials.clone()).await?;
        websocket.use_ns(&namespace).use_db("news").await?;
        websocket
            .query("CREATE auth_probe:record SET value = 1")
            .await?
            .check()?;

        let http = Surreal::new::<Http>(&host).await?;
        http.signin(credentials).await?;
        http.use_ns(&namespace).use_db("news").await?;
        http.query("RETURN 1").await?.check()?;

        // Let the three-second access token expire, without expiring the system session.
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        let error = http
            .query("RETURN 1")
            .await
            .err()
            .context("HTTP unexpectedly accepted the expired token")?;
        ensure!(
            error.to_string().contains("401"),
            "expected HTTP 401: {error}"
        );

        let value: Option<i64> = websocket
            .query(
                "UPDATE auth_probe:record SET value = 2; \
                 SELECT VALUE value FROM ONLY auth_probe:record;",
            )
            .await?
            .check()?
            .take(1)?;
        ensure!(
            value == Some(2),
            "WebSocket read/write failed after token expiry"
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
