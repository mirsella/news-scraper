use std::fs::File;

use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub db_host: String,
    pub db_user: String,
    pub db_pass: String,
    pub headless: bool,
    pub concurrent_tabs: usize,
}

lazy_static! {
    pub static ref CONFIG: Config = load_config();
}

fn load_config() -> Config {
    let file = File::open("config.json").unwrap();
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}
