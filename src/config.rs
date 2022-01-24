use std::path::Path;

use serde::{ Serialize, Deserialize };



#[derive(Serialize, Deserialize)]
pub struct Config {
    pub controller_addr: String,
    pub executor_addr: String,

    pub default_account: Option<String>,
    pub wallet_dir: String,
}
