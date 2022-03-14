use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::bail;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::utils::safe_save;

pub const CLOUD_CLI_CONFIG_FILE_NAME: &str = "config.toml";
pub const CLOUD_CLI_DATA_DIR_NAME: &str = ".cloud-cli";

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    // The dir containing this config file
    #[serde(skip)]
    pub data_dir: PathBuf,

    pub default_context: String,
    pub context_settings: BTreeMap<String, ContextSetting>,
}

impl Config {
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        fs::create_dir_all(data_dir)?;

        let config_file = data_dir.join(CLOUD_CLI_CONFIG_FILE_NAME);
        let mut config = if config_file.exists() {
            let s = fs::read_to_string(config_file)?;
            toml::from_str(&s)?
        } else {
            let mut f = File::create(config_file)?;
            let default_config = Self::default();
            f.write_all(toml::to_string_pretty(&default_config).unwrap().as_bytes())?;

            default_config
        };

        config.data_dir = data_dir.to_path_buf();

        Ok(config)
    }

    // atomically save
    pub fn save(&self) -> Result<()> {
        let path = self.data_dir.join(CLOUD_CLI_CONFIG_FILE_NAME);
        let content = toml::to_string_pretty(self)?;
        safe_save(path, content.as_bytes(), true)
    }
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = {
            let home = home::home_dir().expect("cannot find home dir");
            home.join(CLOUD_CLI_DATA_DIR_NAME)
        };
        let default_context = "default".to_string();
        let context_settings = {
            let mut m = BTreeMap::new();
            m.insert(default_context.clone(), ContextSetting::default());
            m
        };
        Self {
            default_context,
            data_dir,
            context_settings,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CryptoType {
    Sm,
    Eth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSetting {
    pub controller_addr: String,
    pub executor_addr: String,

    pub account_name: String,
    pub crypto_type: CryptoType,
}

impl FromStr for CryptoType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ty = match s.to_uppercase().as_str() {
            "SM" => CryptoType::Sm,
            "ETH" => CryptoType::Eth,
            unknown => bail!("unknown crypto type `{}`", unknown),
        };
        Ok(ty)
    }
}

impl Default for ContextSetting {
    fn default() -> Self {
        Self {
            controller_addr: "localhost:50004".into(),
            executor_addr: "localhost:50002".into(),
            account_name: "default".into(),
            crypto_type: CryptoType::Sm,
        }
    }
}
