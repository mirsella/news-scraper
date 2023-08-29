use std::path::PathBuf;

use shared::*;

#[test]
fn test_load_config_default_path() {
    let config = Config::load("tests/mock_env").unwrap();
    assert_eq!(config.db_user, "news");
    assert_eq!(config.db_password, "arstneoi");
    assert_eq!(config.chrome_headless, None);
    assert_eq!(config.chrome_concurrent_tabs, Some(10));
    assert_eq!(config.chrome_data_dir, Some(PathBuf::from("/tmp/chrome")));
}

#[test]
#[should_panic]
fn test_load_config_invalid_path() {
    let config = Config::load("nonexistent.env");
    dbg!(&config);
    config.unwrap();
}
