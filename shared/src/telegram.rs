use ureq::Response;

pub struct Telegram {
    token: String,
    chat_id: i64,
}

impl Telegram {
    pub fn new(token: impl Into<String>, chat_id: i64) -> Self {
        Telegram {
            token: token.into(),
            chat_id,
        }
    }
    pub fn send(&self, msg: impl Into<String>) -> anyhow::Result<Response> {
        let msg = msg.into();
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let data = serde_json::json!({
            "chat_id": self.chat_id,
            "text": msg,
        });
        Ok(ureq::post(&url).send_json(data)?)
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn it_creates_from_env() {
        dotenvy::dotenv().unwrap();

        let token = env::var("telegram_token").expect("telegram_token not set in env");
        let id = env::var("telegram_id")
            .expect("telegram_id not set in env")
            .parse::<i64>()
            .unwrap();

        let tg = Telegram::new(token, id);
        tg.send("you can ignore this. this is a test for news-scraper::shared::telegram module")
            .unwrap();
    }

    // You can add more tests if needed
}
