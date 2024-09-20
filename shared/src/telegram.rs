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
    #[ignore]
    fn it_creates_from_env() {
        if let Err(e) = dotenvy::dotenv() {
            eprintln!("Error loading .env file: {e}");
        }

        let token = env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not set in env");
        let id = env::var("TELEGRAM_ID")
            .expect("TELEGRAM_ID not set in env")
            .parse::<i64>()
            .unwrap();

        let tg = Telegram::new(token, id);
        tg.send("you can ignore this. this is a test for news-scraper::shared::telegram module")
            .unwrap();
    }

    // You can add more tests if needed
}
