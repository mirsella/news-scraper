use std::path::PathBuf;

use shared::config::Config;

#[test]
fn test_load_config_default_path() {
    let config = Config::load("tests/mock_env").unwrap();
    assert_eq!(config.db_user, "news");
    assert_eq!(config.db_password, "arstneoi");
    assert_eq!(config.openai_api_key, "KEYY");
    assert_eq!(config.article_parser_url, "http://localhost:8080");
    assert_eq!(config.surrealdb_host, "localhost:8000");
    assert_eq!(config.rating_chat_prompt, "test");
    assert_eq!(config.parallel_rating, 10);
    assert_eq!(config.chrome_headless, None);
    assert_eq!(config.chrome_concurrent, Some(10));
    assert_eq!(config.chrome_data_dir, Some(PathBuf::from("/tmp/chrome")));
}

#[test]
#[should_panic]
fn test_load_config_invalid_path() {
    let config = Config::load("nonexistent.env");
    println!("{config:?}");
    config.unwrap();
}
