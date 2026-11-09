//! Best-effort, bounded telemetry. Network failures must not stop news processing.
use serde_json::{Value, json};
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static CLIENT: OnceLock<SyncSender<Message>> = OnceLock::new();
static DROPPED: AtomicU64 = AtomicU64::new(0);
enum Message {
    Event(String, Value),
    Log(String, String, String),
    Flush(SyncSender<()>),
    Stop,
}
pub struct Guard(Option<JoinHandle<()>>);
impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(worker) = self.0.take() {
            if let Some(client) = CLIENT.get() {
                let _ = client.send(Message::Stop);
            }
            let _ = worker.join();
        }
    }
}

pub fn start(service: &'static str) -> Guard {
    let key = std::env::var("POSTHOG_PROJECT_TOKEN").unwrap_or_default();
    if key.is_empty() {
        eprintln!("PostHog disabled: POSTHOG_PROJECT_TOKEN is not set ({service})");
        return Guard(None);
    }
    assert!(
        key.starts_with("phc_"),
        "POSTHOG_PROJECT_TOKEN must be a public project token, not a personal API key"
    );
    let host = std::env::var("POSTHOG_HOST").unwrap_or("https://eu.i.posthog.com".into());
    assert!(host.starts_with("https://"), "POSTHOG_HOST must use HTTPS");
    let environment = std::env::var("POSTHOG_ENVIRONMENT").unwrap_or("production".into());
    let instance = std::env::var("HOSTNAME").unwrap_or_else(|_| std::process::id().to_string());
    let (tx, rx) = mpsc::sync_channel(2048);
    assert!(CLIENT.set(tx).is_ok(), "PostHog initialized more than once");
    let worker = thread::spawn(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(5))
            .build();
        let distinct_id = format!("service:{service}:{instance}");
        let mut heartbeat = Instant::now();
        let event = |name: String, props: Value| {
            let mut properties = props
                .as_object()
                .expect("telemetry properties must be an object")
                .clone();
            properties.extend(
                json!({"distinct_id": distinct_id, "service": service,
                "environment": environment, "$process_person_profile": false})
                .as_object()
                .unwrap()
                .clone(),
            );
            json!({"event": name, "properties": properties, "timestamp": chrono::Utc::now().to_rfc3339()})
        };
        loop {
            let first = rx.recv_timeout(Duration::from_secs(60));
            let mut messages = first.ok().into_iter().collect::<Vec<_>>();
            messages.extend(rx.try_iter().take(99));
            let mut events = Vec::new();
            let mut logs = Vec::new();
            let mut stop = false;
            let mut flushes = Vec::new();
            for message in messages {
                match message {
                    Message::Stop => stop = true,
                    Message::Flush(ack) => flushes.push(ack),
                    Message::Event(name, props) => events.push(event(name, props)),
                    Message::Log(level, target, body) => {
                        let severity = match level.as_str() {
                            "ERROR" => 17,
                            "WARN" => 13,
                            _ => 9,
                        };
                        if severity == 17 {
                            events.push(event(
                                "$exception".into(),
                                json!({"$exception_list": [{"type": target,
                                "value": body, "mechanism": {"type": "log", "handled": true}}]}),
                            ));
                        }
                        logs.push(json!({"timeUnixNano": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().to_string(),
                            "severityNumber": severity, "severityText": level, "body": {"stringValue": body},
                            "attributes": [{"key":"code.namespace","value":{"stringValue":target}}]}));
                    }
                }
            }
            if stop {
                events.push(event("service_stopped".into(), json!({})));
            }
            if heartbeat.elapsed() >= Duration::from_secs(300) {
                events.push(event(
                    "service_heartbeat".into(),
                    json!({"telemetry_dropped": DROPPED.swap(0, Ordering::Relaxed)}),
                ));
                heartbeat = Instant::now();
            }
            let mut requests = Vec::new();
            if !events.is_empty() {
                requests.push(("/batch/", json!({"api_key":key,"batch":events})));
            }
            if !logs.is_empty() {
                requests.push((
                    "/i/v1/logs",
                    json!({"resourceLogs":[{"resource":{"attributes":[
                    {"key":"service.name","value":{"stringValue":service}},
                    {"key":"deployment.environment","value":{"stringValue":environment}}]},
                    "scopeLogs":[{"scope":{"name":"news-scraper"},"logRecords":logs}]}]}),
                ));
            }
            for (path, payload) in requests {
                if let Err(error) = agent
                    .post(&format!("{}{path}", host.trim_end_matches('/')))
                    .set("Authorization", &format!("Bearer {key}"))
                    .send_json(payload)
                {
                    // Do not recurse through the application logger, or print request bodies/keys.
                    eprintln!(
                        "PostHog delivery failed ({service}, {path}): {}",
                        redact(&error.to_string())
                    );
                }
            }
            for ack in flushes {
                let _ = ack.try_send(());
            }
            if stop {
                break;
            }
        }
    });
    capture(
        "service_started",
        json!({"version":env!("CARGO_PKG_VERSION")}),
    );
    Guard(Some(worker))
}

fn send(message: Message) {
    if let Some(client) = CLIENT.get() {
        if client.try_send(message).is_err() {
            let count = DROPPED.fetch_add(1, Ordering::Relaxed);
            if count % 100 == 0 {
                eprintln!(
                    "PostHog queue full or closed: dropping telemetry ({} dropped)",
                    count + 1
                );
            }
        }
    }
}
pub fn flush() {
    if let Some(client) = CLIENT.get() {
        let (ack, received) = mpsc::sync_channel(1);
        if client.try_send(Message::Flush(ack)).is_err()
            || received.recv_timeout(Duration::from_secs(12)).is_err()
        {
            eprintln!("PostHog flush incomplete: queue unavailable or delivery timed out");
        }
    }
}
pub fn capture(event: &str, properties: Value) {
    assert!(
        properties.is_object(),
        "telemetry properties must be an object"
    );
    send(Message::Event(event.into(), properties));
}
pub fn log(record: &log::Record<'_>) {
    // Third-party HTTP debug logs can contain headers and request/response bodies.
    if record.level() > log::Level::Info
        || !["fetcher", "rater", "shared"]
            .iter()
            .any(|p| record.target().starts_with(p))
    {
        return;
    }
    send(Message::Log(
        record.level().to_string(),
        record.target().into(),
        log_body(record),
    ));
}
fn log_body(record: &log::Record<'_>) -> String {
    if record.level() == log::Level::Info {
        redact(&record.args().to_string())
    } else {
        // Database/conversion errors can embed entire records. Regex redaction cannot make them safe.
        format!(
            "{} in {}; details retained in local logs",
            record.level(),
            record.target()
        )
    }
}
fn redact(message: &str) -> String {
    static TOKENS: OnceLock<regex::Regex> = OnceLock::new();
    let mut safe = message.to_string();
    for (name, value) in std::env::vars() {
        if value.len() > 5
            && ["KEY", "TOKEN", "PASSWORD", "SECRET"]
                .iter()
                .any(|suffix| name.ends_with(suffix))
        {
            safe = safe.replace(&value, "[redacted]");
        }
    }
    let re = TOKENS.get_or_init(|| regex::Regex::new(r#"(?i)(?:bearer\s+[^\s,]+|\b(?:sk-|phc_|phx_)[A-Za-z0-9_-]+|eyJ[A-Za-z0-9_.-]+|https?://[^\s"'<>]+)"#).unwrap());
    re.replace_all(&safe, "[redacted]")
        .chars()
        .take(1500)
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn does_not_export_error_payloads() {
        let body = super::log_body(
            &log::Record::builder()
                .level(log::Level::Error)
                .target("fetcher")
                .args(format_args!(
                    "conversion failed: text_body=PRIVATE_ARTICLE_CONTENT"
                ))
                .build(),
        );
        assert!(!body.contains("PRIVATE_ARTICLE_CONTENT"));
        assert!(body.contains("ERROR in fetcher"));
    }
    #[test]
    fn removes_credentials_and_urls() {
        let safe = super::redact(
            "failed https://example.com/?token=secret Bearer abcdef sk-test123 phx_test eyJhbGciOi.x.y",
        );
        assert!(!safe.contains("secret"));
        assert!(!safe.contains("abcdef"));
        assert!(!safe.contains("sk-test123"));
        assert!(!safe.contains("eyJ"));
    }
}
