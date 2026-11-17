use std::path::PathBuf;

use shared::config::{Config, RatingServiceTier};

#[test]
fn test_load_config_default_path() {
    let config = Config::load("tests/mock_env").unwrap();
    assert_eq!(config.db_user, "news");
    assert_eq!(config.db_password, "arstneoi");
    assert_eq!(config.openai_api_key, "KEYY");
    assert_eq!(config.article_parser_url, "http://localhost:8080");
    assert_eq!(config.surrealdb_host, "localhost:8000");
    assert_eq!(config.parallel_rating, 10);
    assert_eq!(config.rating_model, "gpt-6-luna");
    assert_eq!(
        config.rating_reasoning_effort,
        async_openai::types::chat::ReasoningEffort::Low
    );
    assert_eq!(config.rating_max_completion_tokens, 1000);
    assert_eq!(config.rating_service_tier, RatingServiceTier::Standard);
    assert!(!config.rating_reuse_exact_duplicates);
    assert_eq!(config.chrome_headless, None);
    assert_eq!(config.chrome_concurrent, Some(10));
    assert_eq!(config.chrome_data_dir, Some(PathBuf::from("/tmp/chrome")));
}

#[test]
fn rating_optimizations_can_be_enabled_with_env_settings() {
    let settings = [
        ("DB_USER", "unused"),
        ("DB_PASSWORD", "unused"),
        ("OPENAI_API_KEY", "unused"),
        ("ARTICLE_PARSER_URL", "unused"),
        ("SURREALDB_HOST", "unused"),
        ("PARALLEL_RATING", "1"),
        ("TELEGRAM_TOKEN", "unused"),
        ("TELEGRAM_ID", "0"),
        ("RATING_SERVICE_TIER", "flex"),
        ("RATING_REUSE_EXACT_DUPLICATES", "true"),
        ("RATING_REASONING_EFFORT", "none"),
    ];
    let config: Config =
        envy::from_iter(settings.map(|(key, value)| (key.into(), value.into()))).unwrap();
    assert_eq!(config.rating_service_tier, RatingServiceTier::Flex);
    assert!(config.rating_reuse_exact_duplicates);
    assert_eq!(
        config.rating_reasoning_effort,
        async_openai::types::chat::ReasoningEffort::None
    );
}

#[test]
#[should_panic]
fn test_load_config_invalid_path() {
    let config = Config::load("nonexistent.env");
    println!("{config:?}");
    config.unwrap();
}
