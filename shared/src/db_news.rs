use anyhow::{anyhow, Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, FinishReason,
    },
    Client as ChatClient,
};
use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use surrealdb::{engine::remote::ws::Client as DbClient, Surreal};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DbNews {
    // pub id: Option<surrealdb::opt::RecordId>,
    pub id: Option<surrealdb::opt::RecordId>,
    pub html_body: Cow<'static, str>,
    pub text_body: Cow<'static, str>,
    pub caption: Cow<'static, str>,
    pub date: surrealdb::sql::Datetime,
    pub link: Cow<'static, str>,
    pub note: Cow<'static, str>,
    pub provider: Cow<'static, str>,
    pub rating: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub title: Cow<'static, str>,
    pub used: bool,
}

impl DbNews {
    pub async fn save(&self, db: &Surreal<DbClient>) -> Result<DbNews> {
        let id = self.id.clone().unwrap();
        let news = db
            .update::<Option<DbNews>>(("news", id))
            .content(self)
            .await?
            .ok_or(anyhow!("no news found"))?;
        Ok(news)
    }
    pub async fn rate(
        &mut self,
        client: &ChatClient<OpenAIConfig>,
        prompt: impl Into<String>,
    ) -> Result<(u32, Vec<String>)> {
        let text = format!("{}\n{}", &self.title, &self.text_body);
        let tokenizer = tiktoken_rs::p50k_base().unwrap();
        let tokens = tokenizer.encode_with_special_tokens(&text);
        let truncated_tokens = tokens.into_iter().take(500).collect::<Vec<usize>>();
        let truncated_text =
            String::from_utf8_lossy(&tokenizer._decode_native(&truncated_tokens)).to_string();
        // remove the � from lost bytes
        let truncated_text = truncated_text.trim_end_matches('\u{FFFD}').to_string();
        let conv = vec![
            ChatCompletionRequestSystemMessage {
                content: Some(
                    "your response will be exactly in the following format `rating;tags,etc`"
                        .into(),
                ),
                ..Default::default()
            }
            .into(),
            ChatCompletionRequestSystemMessage {
                content: Some(prompt.into()),
                ..Default::default()
            }
            .into(),
            ChatCompletionRequestUserMessage {
                content: Some(truncated_text.into()),
                ..Default::default()
            }
            .into(),
        ];
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .max_tokens(30_u16)
            .messages(conv)
            .n(1)
            .temperature(0_f32)
            .build()
            .unwrap();

        let response = client
            .chat() // Get the API "group" (completions, images, etc.) from the client
            .create(request) // Make the API call in that "group"
            .await
            .context("chat().create(request)")?;
        // println!("cost of tokens {:?}", response.usage.clone().unwrap());
        let choice = response
            .choices
            .first()
            .ok_or(anyhow!("no response. {response:?}"))?;
        let content = choice
            .message
            .content
            .clone()
            .ok_or(anyhow!("no content in response: {response:?}"))?;
        let split = content
            .split_once(';')
            .ok_or(anyhow!("invalid response: {content}"))?;
        let rating = split
            .0
            .trim_start_matches("rating: ")
            .trim_start_matches("Rating: ")
            .parse::<u32>()
            .context(format!("split: {split:?}"))?;
        let mut tags: Vec<String> = split
            .1
            .split(',')
            .filter_map(|s| {
                let s = s.trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            })
            .collect();
        // if response was truncated, remove the last unfinished tag
        if let Some(FinishReason::Length) = &choice.finish_reason {
            _ = tags.pop()
        };
        self.rating = Some(rating);
        match &mut self.tags {
            Some(t) => t.extend(tags.clone()),
            None => self.tags = Some(tags.clone()),
        }
        Ok((rating, tags))
    }
}
