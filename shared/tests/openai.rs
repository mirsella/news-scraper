use anyhow::{Context, Result, ensure};
use async_openai::{Client, config::OpenAIConfig};
use serde_json::{Value, json};
use shared::{Config, DbNews};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::test]
async fn structured_rating_request_and_response() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}/v1", listener.local_addr()?);
    let mut server = tokio::spawn(async move {
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
        let request: Value = serde_json::from_slice(&bytes[header_end..header_end + length])?;
        let response = json!({
            "id": "chatcmpl-test", "object": "chat.completion", "created": 1,
            "model": "gpt-6-luna",
            "choices": [{ "index": 0, "finish_reason": "stop", "message": {
                "role": "assistant", "content": "{\"positivity_rating\":85,\"travel_rating\":30,\"tags\":[\"community\"]}"
            }}],
            "usage": { "prompt_tokens": 650, "completion_tokens": 50, "total_tokens": 700,
                "prompt_tokens_details": { "cached_tokens": 20, "cache_write_tokens": 10 },
                "completion_tokens_details": { "reasoning_tokens": 5 }
            }
        }).to_string();
        socket.write_all(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
            response.len()
        ).as_bytes()).await?;
        Ok::<_, anyhow::Error>(request)
    });

    let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        // Build config without loading .env or changing process-global environment.
        let config: Config = serde_json::from_value(json!({
            "db_user":"unused", "db_password":"unused", "openai_api_key":"test-key",
            "article_parser_url":"unused", "surrealdb_host":"unused", "parallel_rating":1,
            "telegram_token":"unused", "telegram_id":0
        }))?;
        let client = Client::with_config(
            OpenAIConfig::new()
                .with_api_key("test-key")
                .with_api_base(base),
        );
        let mut article = DbNews {
            title: "A community garden".into(),
            text_body: "Des voisins cultivent un jardin ensemble. "
                .repeat(500)
                .into(),
            tags: vec!["fr".into()],
            ..Default::default()
        };
        article
            .rate(&client, "Rate this news article.", &config)
            .await?;
        let request = (&mut server).await??;
        ensure!(request["model"] == "gpt-6-luna");
        ensure!(request["reasoning_effort"] == "low");
        ensure!(request["max_completion_tokens"] == 1000);
        ensure!(request["response_format"]["json_schema"]["strict"] == true);
        ensure!(
            request["response_format"]["json_schema"]["schema"]["additionalProperties"] == false
        );
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
        Ok::<_, anyhow::Error>(())
    })
    .await;
    server.abort();
    outcome.context("mock OpenAI request timed out")?
}
