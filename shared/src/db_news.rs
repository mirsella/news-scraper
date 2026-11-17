use crate::config::RatingServiceTier;
use anyhow::{Context, Result, anyhow};
use async_openai::{
    Client as ChatClient,
    config::OpenAIConfig,
    error::{ApiError, ApiErrorResponse, OpenAIError, WrappedError},
    middleware::HttpRequestFactory,
    types::chat::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, CreateChatCompletionResponse, FinishReason,
        ResponseFormat, ResponseFormatJsonSchema, ServiceTier,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{borrow::Cow, sync::OnceLock, time::Duration};
use surrealdb::{
    Surreal,
    types::{Datetime, RecordId, SurrealValue},
};

#[derive(Debug, Deserialize)]
pub struct RatingResponse {
    positivity_rating: u8,
    travel_rating: u8,
    tags: Vec<String>,
}

pub fn rating_client(config: OpenAIConfig, tier: RatingServiceTier) -> ChatClient<OpenAIConfig> {
    if tier == RatingServiceTier::Standard {
        return ChatClient::with_config(config);
    }
    // The SDK otherwise retries Flex capacity errors on Flex before returning them.
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("build OpenAI HTTP client");
    ChatClient::build(http.clone(), config).with_http_service(tower::service_fn(
        move |request: HttpRequestFactory| {
            let http = http.clone();
            async move {
                let response = http.execute(request.build().await?).await?;
                let status_code = response.status();
                // The SDK loses 408/429 status when a proxy sends a non-JSON body.
                if matches!(status_code.as_u16(), 408 | 429) {
                    let api_error = if status_code.as_u16() == 429 {
                        response
                            .bytes()
                            .await
                            .ok()
                            .and_then(|bytes| serde_json::from_slice::<WrappedError>(&bytes).ok())
                            .map(|body| body.error)
                    } else {
                        None
                    }
                    .unwrap_or_else(|| ApiError {
                        message: status_code.to_string(),
                        ..Default::default()
                    });
                    Err(OpenAIError::ApiError(ApiErrorResponse {
                        status_code,
                        api_error,
                    }))
                } else {
                    Ok(response)
                }
            }
        },
    ))
}

fn is_flex_unavailable(error: &OpenAIError) -> bool {
    let OpenAIError::ApiError(error) = error else {
        return false;
    };
    error.status_code.as_u16() == 429
        && (matches!(
            error.api_error.code.as_deref(),
            Some("flex_unavailable" | "resource_unavailable")
        ) || error.api_error.r#type.as_deref() == Some("resource_unavailable"))
}

pub fn is_temporary_rating_error(error: &anyhow::Error) -> bool {
    match error.downcast_ref::<OpenAIError>() {
        Some(OpenAIError::ApiError(error)) => {
            matches!(error.status_code.as_u16(), 408 | 500 | 502 | 503 | 504)
                || (error.status_code.as_u16() == 429
                    && ![
                        error.api_error.code.as_deref(),
                        error.api_error.r#type.as_deref(),
                    ]
                    .into_iter()
                    .any(|value| {
                        matches!(
                            value,
                            Some("insufficient_quota" | "billing_hard_limit_reached")
                        )
                    }))
        }
        Some(OpenAIError::Reqwest(error)) => error.is_connect() || error.is_timeout(),
        _ => false,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, SurrealValue)]
pub struct DbNews {
    pub id: Option<RecordId>,
    pub html_body: Cow<'static, str>,
    pub text_body: Cow<'static, str>,
    pub caption: Cow<'static, str>,
    pub date: Datetime,
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
        let id = self
            .id
            .clone()
            .context("cannot save news without a record ID")?;
        let news = db
            .update::<Option<DbNews>>(id)
            .content(self.clone())
            .await
            .context("surrealdb error")?
            .ok_or(anyhow!("no news found"))?;
        Ok(news)
    }
    pub fn apply_rating(&mut self, rating: &RatingResponse) {
        self.rating = Some(rating.positivity_rating);
        self.rating_travel = Some(rating.travel_rating);
        self.tags.extend(rating.tags.iter().cloned());
    }

    pub async fn request_rating(
        &self,
        client: &ChatClient<OpenAIConfig>,
        prompt: &str,
        config: &crate::Config,
    ) -> Result<RatingResponse> {
        let text = format!("{}\n{}", self.title, self.text_body);
        static TOKENIZER: OnceLock<tiktoken_rs::CoreBPE> = OnceLock::new();
        let tokenizer =
            TOKENIZER.get_or_init(|| tiktoken_rs::p50k_base().expect("built-in p50k tokenizer"));
        let tokens = tokenizer.encode_with_special_tokens(&text);
        let truncated_bytes = tokenizer.decode_bytes(&tokens[..tokens.len().min(600)])?;
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

        let tier = match config.rating_service_tier {
            RatingServiceTier::Standard => ServiceTier::Default,
            RatingServiceTier::Flex => ServiceTier::Flex,
        };
        let mut request = CreateChatCompletionRequestArgs::default()
            .model(&config.rating_model)
            .max_completion_tokens(config.rating_max_completion_tokens)
            .reasoning_effort(config.rating_reasoning_effort.clone())
            .service_tier(tier.clone())
            .messages(conv)
            .response_format(ResponseFormat::JsonSchema {
                json_schema: ResponseFormatJsonSchema {
                    name: "article_rating".to_string(),
                    description: Some("Structured rating response for news articles".to_string()),
                    schema,
                    strict: Some(true),
                },
            })
            .build()
            .context("building rating request")?;

        // Keep the raw response for usage fields not exposed by the SDK (e.g. cache writes).
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
                "requested_service_tier": tier,
        });
        let chat = client.chat();
        let mut completion: std::result::Result<serde_json::Value, _> =
            chat.create_byot(&request).await;
        let fallback =
            tier == ServiceTier::Flex && completion.as_ref().is_err_and(is_flex_unavailable);
        if fallback {
            log::debug!("OpenAI Flex capacity unavailable; retrying rating once on Standard");
            request.service_tier = Some(ServiceTier::Default);
            completion = chat.create_byot(&request).await;
        }
        generation["flex_fallback"] = json!(fallback);
        generation["$ai_latency"] = json!(started.elapsed().as_secs_f64());
        let raw = match completion {
            Ok(raw) => raw,
            Err(error) => {
                generation["$ai_is_error"] = json!(true);
                if let OpenAIError::ApiError(error) = &error {
                    generation["$ai_http_status"] = json!(error.status_code.as_u16());
                    generation["api_error_code"] = json!(error.api_error.code);
                }
                generation["$ai_error"] = json!("OpenAI request failed; see local rater logs");
                crate::telemetry::capture("$ai_generation", generation);
                return Err(error).context("requesting OpenAI rating");
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
        let served_tier = raw["service_tier"].as_str();
        generation["served_service_tier"] = json!(served_tier);
        // An omitted tier on a Flex response does not prove it was billed at Flex rates.
        let pricing_tier = served_tier
            .or((request.service_tier == Some(ServiceTier::Default)).then_some("default"));
        if let Some((input, output)) = pricing_tier
            .and_then(|tier| generation_costs(&config.rating_model, tier, &raw["usage"]))
        {
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

        let rating_response: RatingResponse = serde_json::from_str(content)
            .context("failed to parse rating JSON (response body omitted)")?;

        anyhow::ensure!(
            rating_response.positivity_rating <= 100 && rating_response.travel_rating <= 100,
            "rating outside 0..=100"
        );

        Ok(rating_response)
    }
}

// Short-context USD prices per million tokens. Output includes reasoning.
fn generation_costs(model: &str, tier: &str, usage: &serde_json::Value) -> Option<(f64, f64)> {
    let multiplier = match tier {
        "default" => 1.0,
        "flex" => 0.5,
        _ => {
            log::warn!("No explicit cost estimate configured for OpenAI service tier {tier}");
            return None;
        }
    };
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
        multiplier
            * (uncached as f64 * input_price
                + cached as f64 * cache_price
                + written as f64 * write_price)
            / 1_000_000.0,
        multiplier * output as f64 * output_price / 1_000_000.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luna_cost_includes_cache_writes_and_reasoning_output() {
        let usage = json!({"prompt_tokens":1000,"completion_tokens":100,"prompt_tokens_details":{"cached_tokens":100,"cache_write_tokens":100}});
        let (input, output) = generation_costs("gpt-6-luna", "default", &usage).unwrap();
        assert!((input - 0.0000935).abs() < 1e-12);
        assert!((output - 0.00005).abs() < 1e-12);
        let (flex_input, flex_output) = generation_costs("gpt-6-luna", "flex", &usage).unwrap();
        assert!((flex_input - input / 2.0).abs() < 1e-12);
        assert!((flex_output - output / 2.0).abs() < 1e-12);
    }
}
