use std::path::Path;

use serde::{ Serialize, Deserialize };

use crate::sdk::wallet::WalletConfig;


#[derive(Serialize, Deserialize)]
pub struct Config {
    pub wallet: WalletConfig,
}
