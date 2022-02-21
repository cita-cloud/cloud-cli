use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::io::Write;
use std::fs;
use std::fs::File;

use tempfile::NamedTempFile;

const CLOUD_CLI_CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    // The dir containing this config file
    #[serde(skip)]
    pub data_dir: PathBuf,

    pub default_context: String,
    pub contexts: Vec<ContextConfig>,
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
        let tmp = NamedTempFile::new_in(self.data_dir)?;
        let content = toml::to_string_pretty(self)?;
        tmp.write_all(content.as_bytes())?;
        let f = tmp.persist(Path::new(&self.data_dir).join(CLOUD_CLI_CONFIG_FILE_NAME))?;
        f.flush()?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = {
            let home = home::home_dir().expect("cannot find home dir");
            home.join(".cloud-cli-v0.3.0")
        };
        Self {
            default_context: "default".into(),
            data_dir,
            contexts: vec![ContextConfig::default()],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContextConfig {
    name: String,
    controller_addr: String,
    executor_addr: String,
    account_id: String,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            name: "default".into(),
            controller_addr: "localhost:50004".into(),
            executor_addr: "localhost:50002".into(),
            account_id: "default".into(),
        }
    }
}
