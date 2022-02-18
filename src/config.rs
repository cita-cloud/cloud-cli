use std::path::Path;

use serde::{Deserialize, Serialize};
use anyhow::Result;

mod default {
    pub fn controller_addr() -> String {
        "localhost:50005".into()
    }

    pub fn executor_addr() -> String {
        "localhost:50002".into()
    }

    pub fn default_account() -> String {
        "default".into()
    }

    pub fn data_dir() -> String {
        let home = home::home_dir().expect("cannot find home dir");
        home.join(".cloud-cli-v0.3.0").to_string_lossy().to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default::controller_addr")]
    pub controller_addr: String,
    #[serde(default = "default::executor_addr")]
    pub executor_addr: String,

    #[serde(default = "default::default_account")]
    pub default_account: String,
    #[serde(default = "default::data_dir")]
    pub data_dir: String,
}
