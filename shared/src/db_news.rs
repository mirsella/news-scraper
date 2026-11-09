use anyhow::{Context, Result, anyhow};
use async_openai::{
    Client as ChatClient,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, CreateChatCompletionResponse, FinishReason,
        ResponseFormat, ResponseFormatJsonSchema,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{borrow::Cow, sync::OnceLock};
use surrealdb::Surreal;

#[derive(Debug, Deserialize)]
struct RatingResponse {
    positivity_rating: u8,
    travel_rating: u8,
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DbNews {
    pub id: Option<surrealdb::opt::RecordId>,
    pub html_body: Cow<'static, str>,
    pub text_body: Cow<'static, str>,
    pub caption: Cow<'static, str>,
    pub date: surrealdb::sql::Datetime,
    pub link: Cow<'static, str>,
    pub note: Cow<'static, str>,
    pub provider: Cow<'static, str>,
    pub rating: Option<u8>,
    pub rating_travel: Option<u8>,
    pub tags: Vec<String>,
    pub title: Cow<'static, str>,
    pub used: bool,
}

impl DbNews {
    pub async fn save<T: surrealdb::Connection>(&self, db: &Surreal<T>) -> Result<DbNews> {
        let id = self.id.clone().unwrap();
        let news = db
            .update::<Option<DbNews>>(("news", id))
            .content(self)
            .await
            .context("surrealdb error")?
            .ok_or(anyhow!("no news found"))?;
        Ok(news)
    }
    pub async fn rate(
        &mut self,
        client: &ChatClient<OpenAIConfig>,
        prompt: &str,
        config: &crate::Config,
    ) -> Result<()> {
        let text = format!("{}\n{}", &self.title, &self.text_body);
        static TOKENIZER: OnceLock<tiktoken_rs::CoreBPE> = OnceLock::new();
        let tokenizer =
            TOKENIZER.get_or_init(|| tiktoken_rs::p50k_base().expect("built-in p50k tokenizer"));
        let tokens = tokenizer.encode_with_special_tokens(&text);
        let truncated_bytes = tokenizer._decode_native(&tokens[..tokens.len().min(600)]);
        let truncated_text = String::from_utf8_lossy(&truncated_bytes);
        // remove the � from lost bytes
        let truncated_text = truncated_text.trim_end_matches('\u{FFFD}').to_string();

        let conv = vec![
            ChatCompletionRequestSystemMessage {
                content: prompt.into(),
                ..Default::default()
            }
            .into(),
            ChatCompletionRequestUserMessage {
                content: truncated_text.into(),
                ..Default::default()
            }
            .into(),
        ];

        let schema = json!({
            "type": "object",
            "properties": {
                "positivity_rating": {
                    "type": "integer",
                    "description": "Positivity rating from 0-100",
                    "minimum": 0,
                    "maximum": 100
                },
                "travel_rating": {
                    "type": "integer",
                    "description": "Travel-related rating from 0-100",
                    "minimum": 0,
                    "maximum": 100
                },
                "tags": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Array of relevant tags for the article"
                }
            },
            "required": ["positivity_rating", "travel_rating", "tags"],
            "additionalProperties": false
        });

        let request = CreateChatCompletionRequestArgs::default()
            .model(&config.rating_model)
            .max_completion_tokens(config.rating_max_completion_tokens)
            .messages(conv)
            .response_format(ResponseFormat::JsonSchema {
                json_schema: ResponseFormatJsonSchema {
                    name: "article_rating".to_string(),
                    description: Some("Structured rating response for news articles".to_string()),
                    schema: Some(schema),
                    strict: Some(true),
                },
            })
            .build()
            .unwrap();

        // The SDK's old enum predates Luna's `none` and `xhigh` efforts.
        let mut request = serde_json::to_value(request)?;
        request["reasoning_effort"] = json!(config.rating_reasoning_effort);
        let started = std::time::Instant::now();
        let trace_id = format!(
            "rating-{}-{}",
            std::process::id(),
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .expect("current date fits nanoseconds")
        );
        let mut generation = json!({
                "$ai_trace_id":trace_id, "$ai_session_id":null,
                "$ai_span_name":"article_rating", "$ai_stream":false,
                "$ai_max_tokens":config.rating_max_completion_tokens,
                "$ai_provider":"openai", "$ai_model":config.rating_model,
                "reasoning_effort":config.rating_reasoning_effort,
                "provider":self.provider,
        });
        let completion: std::result::Result<serde_json::Value, _> =
            client.chat().create_byot(request).await;
        generation["$ai_latency"] = json!(started.elapsed().as_secs_f64());
        let raw = match completion {
            Ok(raw) => raw,
            Err(error) => {
                generation["$ai_is_error"] = json!(true);
                generation["$ai_error"] = json!("OpenAI request failed; see local rater logs");
                crate::telemetry::capture("$ai_generation", generation);
                return Err(error).context("created openai request");
            }
        };
        generation["$ai_http_status"] = json!(200);
        generation["$ai_is_error"] = json!(false);
        generation["$ai_input_tokens"] = raw["usage"]["prompt_tokens"].clone();
        generation["$ai_output_tokens"] = raw["usage"]["completion_tokens"].clone();
        generation["$ai_cache_read_input_tokens"] =
            raw["usage"]["prompt_tokens_details"]["cached_tokens"].clone();
        generation["$ai_cache_creation_input_tokens"] =
            raw["usage"]["prompt_tokens_details"]["cache_write_tokens"].clone();
        generation["reasoning_tokens"] =
            raw["usage"]["completion_tokens_details"]["reasoning_tokens"].clone();
        generation["$ai_stop_reason"] = raw["choices"][0]["finish_reason"].clone();
        if let Some((input, output)) = generation_costs(&config.rating_model, &raw["usage"]) {
            generation["$ai_input_cost_usd"] = json!(input);
            generation["$ai_output_cost_usd"] = json!(output);
            generation["$ai_total_cost_usd"] = json!(input + output);
        }
        crate::telemetry::capture("$ai_generation", generation);
        let response: CreateChatCompletionResponse = serde_json::from_value(raw)?;

        let choice = response
            .choices
            .first()
            .ok_or(anyhow!("OpenAI returned no choices"))?;

        anyhow::ensure!(
            choice.finish_reason != Some(FinishReason::Length),
            "rating output truncated; increase RATING_MAX_COMPLETION_TOKENS"
        );

        let content = choice
            .message
            .content
            .as_deref()
            .ok_or(anyhow!("OpenAI returned no rating content"))?;

        let rating_response: RatingResponse = serde_json::from_str(&content)
            .context("failed to parse rating JSON (response body omitted)")?;

        anyhow::ensure!(
            rating_response.positivity_rating <= 100 && rating_response.travel_rating <= 100,
            "rating outside 0..=100"
        );

        self.rating = Some(rating_response.positivity_rating);
        self.rating_travel = Some(rating_response.travel_rating);
        self.tags.extend(rating_response.tags);
        Ok(())
    }
}

// Standard USD prices per million tokens, verified 2026-11-09. Output includes reasoning.
fn generation_costs(model: &str, usage: &serde_json::Value) -> Option<(f64, f64)> {
    let (input_price, cache_price, write_price, output_price) = match model {
        "gpt-6-luna" => (0.10, 0.01, 0.125, 0.50),
        "gpt-5-nano" => (0.05, 0.005, 0.05, 0.40),
        _ => {
            log::warn!("No explicit cost estimate configured for rating model {model}");
            return None;
        }
    };
    let input = usage["prompt_tokens"].as_u64()?;
    let output = usage["completion_tokens"].as_u64()?;
    let cached = usage["prompt_tokens_details"]["cached_tokens"]
        .as_u64()
        .unwrap_or(0);
    let written = usage["prompt_tokens_details"]["cache_write_tokens"]
        .as_u64()
        .unwrap_or(0);
    let Some(uncached) = input.checked_sub(cached + written) else {
        log::error!("Invalid OpenAI usage: cache token counts exceed input token count");
        return None;
    };
    Some((
        (uncached as f64 * input_price
            + cached as f64 * cache_price
            + written as f64 * write_price)
            / 1_000_000.0,
        output as f64 * output_price / 1_000_000.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luna_cost_includes_cache_writes_and_reasoning_output() {
        let usage = json!({"prompt_tokens":1000,"completion_tokens":100,"prompt_tokens_details":{"cached_tokens":100,"cache_write_tokens":100}});
        let (input, output) = generation_costs("gpt-6-luna", &usage).unwrap();
        assert!((input - 0.0000935).abs() < 1e-12);
        assert!((output - 0.00005).abs() < 1e-12);
    }
}
