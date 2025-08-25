use anyhow::{anyhow, Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, FinishReason, ReasoningEffort, ResponseFormat,
        ResponseFormatJsonSchema,
    },
    Client as ChatClient,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
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
    ) -> Result<(u8, u8, Vec<String>)> {
        let text = format!("{}\n{}", &self.title, &self.text_body);
        let tokenizer = tiktoken_rs::p50k_base().unwrap();
        let tokens = tokenizer.encode_with_special_tokens(&text);
        let truncated_tokens = tokens.into_iter().take(600).collect::<Vec<usize>>();
        let truncated_text =
            String::from_utf8_lossy(&tokenizer._decode_native(&truncated_tokens)).to_string();
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
            .model("gpt-5-nano")
            .max_completion_tokens(1000u32)
            .reasoning_effort(ReasoningEffort::Low)
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

        let response = client
            .chat()
            .create(request)
            .await
            .context("created openai request")?;

        let choice = response
            .choices
            .first()
            .ok_or(anyhow!("no response. {response:?}"))?;

        let content = choice
            .message
            .content
            .clone()
            .ok_or(anyhow!("no content in response: {response:?}"))?;

        let rating_response: RatingResponse = serde_json::from_str(&content)
            .context(format!("failed to parse response JSON: {}", content))?;

        let mut tags = rating_response.tags;
        // if response was truncated, remove the last unfinished tag
        if let Some(FinishReason::Length) = &choice.finish_reason {
            println!("db_news.rate(): response was truncated");
            _ = tags.pop();
        };

        self.rating = Some(rating_response.positivity_rating);
        self.rating_travel = Some(rating_response.travel_rating);
        self.tags.extend(tags.clone());

        Ok((rating_response.positivity_rating, rating_response.travel_rating, tags))
    }
}
