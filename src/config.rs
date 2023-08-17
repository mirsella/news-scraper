use anyhow::Context;
use serde::Deserialize;
use std::fs::File;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub db_host: String,
    pub db_user: String,
    pub db_pass: String,
    pub headless: bool,
    pub concurrent_tabs: usize,
}

pub fn load_config(path: Option<&str>) -> Config {
    let file = File::open(path.unwrap_or("config.json"))
        .context("opening config.json")
        .unwrap();
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}
