pub mod account;
pub mod admin;
pub mod context;
pub mod controller;
pub mod evm;
pub mod executor;
pub mod wallet;

use crate::crypto::Crypto;
use clap::{App, ArgMatches};
use context::Context;
use std::collections::HashMap;

use anyhow::{bail, ensure, Context as _, Result};
