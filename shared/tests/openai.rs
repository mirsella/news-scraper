use anyhow::{Context, Result, ensure};
use async_openai::config::OpenAIConfig;
use serde_json::{Value, json};
use shared::{
    Config, DbNews,
    config::RatingServiceTier,
    db_news::{is_temporary_rating_error, rating_client},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn rate_with_mock(
    tier: RatingServiceTier,
    replies: Vec<(&'static str, Value)>,
) -> Result<(DbNews, Vec<Value>, Result<()>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}/v1", listener.local_addr()?);
    let mut server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, reply) in replies {
            let (mut socket, _) = listener.accept().await?;
            let mut bytes = Vec::new();
            let header_end = loop {
                let mut buffer = [0; 4096];
                let read = socket.read(&mut buffer).await?;
                ensure!(read > 0, "request ended before its headers");
                bytes.extend_from_slice(&buffer[..read]);
                if let Some(index) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    break index + 4;
                }
            };
            let headers = std::str::from_utf8(&bytes[..header_end])?;
            ensure!(headers.starts_with("POST /v1/chat/completions HTTP/1.1"));
            let length: usize = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then_some(value.trim())
                })
                .context("request lacks Content-Length")?
                .parse()?;
            while bytes.len() < header_end + length {
                let mut buffer = [0; 4096];
                let read = socket.read(&mut buffer).await?;
                ensure!(read > 0, "request body ended early");
                bytes.extend_from_slice(&buffer[..read]);
            }
            requests.push(serde_json::from_slice(
                &bytes[header_end..header_end + length],
            )?);
            let truncated = reply.is_null();
            let response = match reply {
                Value::Null => "<html>".into(),
                Value::String(raw) => raw,
                reply => reply.to_string(),
            };
            socket.write_all(format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
            response.len() + usize::from(truncated)
        ).as_bytes()).await?;
        }
        Ok::<_, anyhow::Error>(requests)
    });

    let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        // Build config without loading .env or changing process-global environment.
        let mut config: Config = serde_json::from_value(json!({
            "db_user":"unused", "db_password":"unused", "openai_api_key":"test-key",
            "article_parser_url":"unused", "surrealdb_host":"unused", "parallel_rating":1,
            "telegram_token":"unused", "telegram_id":0
        }))?;
        config.rating_service_tier = tier;
        let client = rating_client(
            OpenAIConfig::new()
                .with_api_key("test-key")
                .with_api_base(base),
            tier,
        );
        let mut article = DbNews {
            title: "A community garden".into(),
            text_body: "Des voisins cultivent un jardin ensemble. "
                .repeat(500)
                .into(),
            tags: vec!["fr".into()],
            ..Default::default()
        };
        let outcome = article
            .request_rating(&client, "Rate this news article.", &config)
            .await
            .map(|rating| article.apply_rating(&rating));
        Ok((article, (&mut server).await??, outcome))
    })
    .await;
    server.abort();
    outcome.context("mock OpenAI request timed out")?
}

fn success(tier: &str) -> (&'static str, Value) {
    (
        "200 OK",
        json!({
            "id": "chatcmpl-test", "object": "chat.completion", "created": 1,
            "model": "gpt-6-luna", "service_tier": tier,
            "choices": [{ "index": 0, "finish_reason": "stop", "message": {
                "role": "assistant", "content": "{\"positivity_rating\":85,\"travel_rating\":30,\"tags\":[\"community\"]}"
            }}],
            "usage": { "prompt_tokens": 650, "completion_tokens": 50, "total_tokens": 700,
                "prompt_tokens_details": { "cached_tokens": 20, "cache_write_tokens": 10 },
                "completion_tokens_details": { "reasoning_tokens": 5 }
            }
        }),
    )
}

fn capacity_error() -> (&'static str, Value) {
    api_error("resource_unavailable", Some("flex_unavailable"))
}

fn api_error(kind: &str, code: Option<&str>) -> (&'static str, Value) {
    (
        "429 Too Many Requests",
        json!({"error": {"message": "test API error", "type": kind, "code": code}}),
    )
}

#[tokio::test]
async fn structured_rating_request_and_response() -> Result<()> {
    let (article, requests, result) =
        rate_with_mock(RatingServiceTier::Standard, vec![success("default")]).await?;
    result?;
    let request = &requests[0];
    ensure!(request["service_tier"] == "default");
    ensure!(request["model"] == "gpt-6-luna");
    ensure!(request["reasoning_effort"] == "low");
    ensure!(request["max_completion_tokens"] == 1000);
    ensure!(request["response_format"]["json_schema"]["strict"] == true);
    ensure!(request["response_format"]["json_schema"]["schema"]["additionalProperties"] == false);
    let text = request["messages"][1]["content"]
        .as_str()
        .context("missing article text")?;
    ensure!(
        !text.ends_with('\u{FFFD}'),
        "truncation left an incomplete UTF-8 character"
    );
    ensure!(
        tiktoken_rs::p50k_base()?
            .encode_with_special_tokens(text)
            .len()
            <= 600
    );
    ensure!(article.rating == Some(85) && article.rating_travel == Some(30));
    ensure!(article.tags == ["fr", "community"]);
    Ok(())
}

#[tokio::test]
async fn flex_capacity_failure_retries_once_on_standard() -> Result<()> {
    let (article, mut requests, result) = rate_with_mock(
        RatingServiceTier::Flex,
        vec![capacity_error(), success("default")],
    )
    .await?;
    result?;
    ensure!(article.rating == Some(85));
    ensure!(requests[0]["service_tier"] == "flex" && requests[1]["service_tier"] == "default");
    requests[0]["service_tier"] = json!("default");
    ensure!(
        requests[0] == requests[1],
        "fallback changed more than the service tier"
    );
    Ok(())
}

#[tokio::test]
async fn successful_flex_call_does_not_fall_back() -> Result<()> {
    let (article, requests, result) =
        rate_with_mock(RatingServiceTier::Flex, vec![success("flex")]).await?;
    result?;
    ensure!(article.rating == Some(85) && requests[0]["service_tier"] == "flex");
    Ok(())
}

#[tokio::test]
async fn ordinary_rate_limit_and_quota_errors_do_not_fall_back() -> Result<()> {
    for (kind, code, temporary) in [
        ("rate_limit_error", Some("rate_limit_exceeded"), true),
        ("rate_limit_error", None, true),
        ("insufficient_quota", Some("insufficient_quota"), false),
    ] {
        let (article, _, result) =
            rate_with_mock(RatingServiceTier::Flex, vec![api_error(kind, code)]).await?;
        ensure!(article.rating.is_none());
        let error = result.expect_err("the API rejected the request");
        // An unexpected fallback would get a connection error from the closed mock server.
        ensure!(format!("{error:#}").contains(code.unwrap_or(kind)));
        ensure!(is_temporary_rating_error(&error) == temporary);
    }
    Ok(())
}

#[tokio::test]
async fn proxy_timeouts_and_throttles_without_json_are_retryable() -> Result<()> {
    for (status, body) in [
        ("408 Request Timeout", json!("<html>proxy timeout</html>")),
        (
            "429 Too Many Requests",
            json!("<html>proxy throttle</html>"),
        ),
        ("429 Too Many Requests", Value::Null),
    ] {
        let (article, _, result) =
            rate_with_mock(RatingServiceTier::Flex, vec![(status, body)]).await?;
        ensure!(article.rating.is_none());
        let error = result.expect_err("the proxy rejected the request");
        ensure!(format!("{error:#}").contains(status));
        ensure!(is_temporary_rating_error(&error));
    }
    Ok(())
}

#[tokio::test]
async fn a_rate_limit_on_fallback_is_still_retryable() -> Result<()> {
    let (article, requests, result) = rate_with_mock(
        RatingServiceTier::Flex,
        vec![
            capacity_error(),
            api_error("rate_limit_error", Some("rate_limit_exceeded")),
        ],
    )
    .await?;
    ensure!(article.rating.is_none());
    ensure!(requests[1]["service_tier"] == "default");
    let error = result.expect_err("the API rejected the fallback");
    ensure!(format!("{error:#}").contains("rate_limit_exceeded"));
    ensure!(is_temporary_rating_error(&error));
    Ok(())
}
