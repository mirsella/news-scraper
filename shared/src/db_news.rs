use anyhow::{anyhow, Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequestArgs, Role},
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
    // pub async fn new_nonrated(db: &Surreal<DbClient>) -> Result<DbNews> {
    //     let news: Option<DbNews> = db
    //         .query(
    //             "select * from news where rating == none AND date > time::floor(time::now(), 1w) limit 1",
    //         )
    //         .await?
    //         .take(0)?;
    //     news.ok_or(anyhow!("no news found"))
    // }

    pub async fn save(&self, db: &Surreal<DbClient>) -> Result<()> {
        let id = self.id.clone().unwrap();
        println!("saving news with tags: {:?}", self.tags);
        let news = db
            .update::<Option<DbNews>>(("news", id))
            .content(self)
            .await?
            .ok_or(anyhow!("no news found"))?;
        println!("saved news with tags: {:?}", news.tags);
        Ok(())
    }
    pub async fn rate(
        &mut self,
        client: &ChatClient<OpenAIConfig>,
        prompt: &str,
    ) -> Result<(u32, Vec<String>)> {
        let text = self.text_body.clone().to_string();
        let tokenizer = tiktoken_rs::p50k_base().unwrap();
        let tokens = tokenizer.encode_with_special_tokens(&text);
        let truncated_tokens = tokens.into_iter().take(500).collect::<Vec<usize>>();
        let truncated_text =
            String::from_utf8_lossy(&tokenizer._decode_native(&truncated_tokens)).to_string();
        // remove the � from lost bytes
        let truncated_text = truncated_text.trim_end_matches('\u{FFFD}').to_string();
        let conv = vec![
            ChatCompletionRequestMessage {
                role: Role::System,
                content: Some(prompt.into()),
                ..Default::default()
            },
            ChatCompletionRequestMessage {
                role: Role::System,
                content: Some(
                    "your output will ONLY be in this format: rating;tags,tags,etc...".to_string(),
                ),
                ..Default::default()
            },
            ChatCompletionRequestMessage {
                role: Role::User,
                content: Some(truncated_text),
                ..Default::default()
            },
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
            .await?;
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
            .ok_or(anyhow!("no rating in response. {content}"))?;
        let rating = split.0.parse::<u32>().context(content.clone())?;
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
        match &choice.finish_reason {
            Some(reason) if reason == "length" => _ = tags.pop(),
            _ => (),
        };
        self.rating = Some(rating);
        self.tags = Some(tags.clone());
        Ok((rating, tags))
    }
}
