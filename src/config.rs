use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub controller_addr: String,
    pub executor_addr: String,

    pub default_account: Option<String>,
    pub wallet_dir: String,
}
