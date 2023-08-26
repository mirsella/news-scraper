use std::path::PathBuf;

use shared::*;

#[test]
fn test_load_config_default_path() {
    let config = load_config(Some("tests/mock_config.toml")).unwrap();
    assert_eq!(config.db.host, "custom_db_host");
    assert_eq!(config.db.user, "custom_db_user");
    assert_eq!(config.db.password, "custom_db_password");
    assert_eq!(config.chrome.headless, None);
    assert_eq!(config.chrome.concurrent_tabs, Some(4));
    assert_eq!(config.chrome.data_dir, Some(PathBuf::from("/path/to/data")));
}

#[test]
#[should_panic]
fn test_load_config_invalid_path() {
    let config = load_config(Some("nonexistent.toml"));
    if let Err(e) = config {
        if e.to_string() == "No such file or directory (os error 2)" {
            panic!("got the right error");
        }
    }
}
