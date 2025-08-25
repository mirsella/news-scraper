use anyhow::{anyhow, Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, FinishReason, ReasoningEffort,
    },
    Client as ChatClient,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use surrealdb::Surreal;

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
            ChatCompletionRequestSystemMessage {
                content:
                // "you're a expert journalist. you will answer with exactly the following format `rating1,rating2;tags,tags,tags`. directly put the values."
                "the article is finished. you will answer with exactly the following format `rating1,rating2;tags,tags,tags`. directly put the values:"
                    .into(),
                ..Default::default()
            }
            .into(),
        ];
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-5-nano")
            .max_completion_tokens(150u32)
            .reasoning_effort(ReasoningEffort::Low)
            .messages(conv)
            .build()
            .unwrap();

        let response = client
            .chat() // Get the API "group" (completions, images, etc.) from the client
            .create(request) // Make the API call in that "group"
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
        let split = content
            .split_once(';')
            .ok_or(anyhow!("invalid response: `{content}`"))?;
        let ratings: (u8, u8) = {
            let lowercase = split.0.to_lowercase();
            let r = lowercase
                .trim_start_matches("rating: ")
                .split_once(',')
                .context(content.clone())?;
            (
                r.0.trim().parse().context(content.clone())?,
                r.1.trim().parse().context(content.clone())?,
            )
        };
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
            println!("db_news.rate(): response was truncated");
            _ = tags.pop();
        };
        self.rating = Some(ratings.0);
        self.rating_travel = Some(ratings.1);
        self.tags.extend(tags.clone());
        Ok((ratings.0, ratings.1, tags))
    }
}
