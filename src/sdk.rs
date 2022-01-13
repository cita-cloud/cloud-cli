pub mod admin;
pub mod controller;
pub mod executor;
#[cfg(feature = "evm")]
pub mod evm;
pub mod account;
pub mod wallet;
pub mod context;

use clap::{App, ArgMatches};
use std::collections::HashMap;
use crate::crypto::Crypto;
use context::Context;

use anyhow::{
    bail, ensure, Context as _, Result
};

