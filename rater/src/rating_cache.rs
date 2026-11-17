use anyhow::Result;
use sha2::{Digest, Sha256};
use shared::db_news::{DbNews, RatingResponse};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

type Key = [u8; 32];
type Entry = Arc<OnceCell<RatingResponse>>;
const MAX_ENTRIES: usize = 16_384;
const MIN_BODY_CHARS: usize = 300;

#[derive(Default)]
struct Entries {
    values: HashMap<Key, Entry>,
    order: VecDeque<Key>,
}

#[derive(Default)]
pub struct RatingCache(Mutex<Entries>);

impl RatingCache {
    pub async fn rate(
        &self,
        news: &mut DbNews,
        request: impl AsyncFnOnce(&DbNews) -> Result<RatingResponse>,
    ) -> Result<bool> {
        let Some(key) = article_key(news) else {
            let rating = request(news).await?;
            news.apply_rating(&rating);
            return Ok(false);
        };
        let entry = self.entry(key);
        // Concurrent copies share one successful response. Errors leave the cell empty.
        let mut reused = true;
        let rating = entry
            .get_or_try_init(|| {
                reused = false;
                request(news)
            })
            .await?;
        news.apply_rating(rating);
        Ok(reused)
    }

    fn entry(&self, key: Key) -> Entry {
        let mut cache = self.0.lock().expect("rating cache lock poisoned");
        if let Some(entry) = cache.values.get(&key) {
            return entry.clone();
        }
        if cache.values.len() == MAX_ENTRIES {
            let oldest = cache
                .order
                .pop_front()
                .expect("full cache has an oldest key");
            cache.values.remove(&oldest);
        }
        let entry = Entry::default();
        cache.values.insert(key, entry.clone());
        cache.order.push_back(key);
        entry
    }
}

fn article_key(news: &DbNews) -> Option<Key> {
    // Short extracts can be the same paywall notice rather than the same story.
    if news.text_body.chars().take(MIN_BODY_CHARS).count() < MIN_BODY_CHARS {
        return None;
    }
    let mut hash = Sha256::new();
    hash.update((news.title.len() as u64).to_be_bytes());
    hash.update(news.title.as_bytes());
    hash.update(news.text_body.as_bytes());
    Some(hash.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn news() -> DbNews {
        DbNews {
            title: "A community garden".into(),
            text_body: "The townspeople tend the garden. ".repeat(20).into(),
            ..Default::default()
        }
    }

    fn response() -> RatingResponse {
        serde_json::from_value(serde_json::json!({
            "positivity_rating": 85, "travel_rating": 3, "tags": ["garden"]
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn concurrent_copies_share_the_rating_but_keep_source_tags() -> Result<()> {
        let mut left = news();
        left.tags.push("be".into());
        let mut right = news();
        right.tags.push("fr".into());
        right.link = "https://another.example".into();
        let cache = RatingCache::default();
        let calls = Cell::new(0);
        let request = async |_: &DbNews| {
            calls.set(calls.get() + 1);
            tokio::task::yield_now().await;
            Ok(response())
        };
        let (a, b) = tokio::join!(
            cache.rate(&mut left, request),
            cache.rate(&mut right, request)
        );
        assert_ne!(a?, b?);
        assert_eq!(calls.get(), 1);
        assert_eq!(left.rating, right.rating);
        assert_eq!(left.rating_travel, right.rating_travel);
        assert_eq!(left.tags, ["be", "garden"]);
        assert_eq!(right.tags, ["fr", "garden"]);
        Ok(())
    }

    #[tokio::test]
    async fn failed_rating_is_not_reused() -> Result<()> {
        let cache = RatingCache::default();
        let mut article = news();
        let error = cache
            .rate(&mut article, async |_| anyhow::bail!("no capacity"))
            .await;
        assert!(error.is_err());
        assert!(article.rating.is_none());
        assert!(!cache.rate(&mut article, async |_| Ok(response())).await?);
        Ok(())
    }

    #[test]
    fn key_uses_full_text_and_title_and_skips_short_bodies() {
        let article = news();
        let mut edit = article.clone();
        edit.title.to_mut().push('!');
        assert_ne!(article_key(&article), article_key(&edit));
        edit.title = article.title.clone();
        edit.text_body.to_mut().push('!');
        assert_ne!(article_key(&article), article_key(&edit));
        edit.text_body = "paywall".into();
        assert!(article_key(&edit).is_none());
    }
}
