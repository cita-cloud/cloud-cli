// This is from [ethabi-cli](https://github.com/rust-ethereum/ethabi).
// I modified the command line parser to integrate it into cloud-cli.

use anyhow::anyhow;
use clap::Arg;

use ethabi::{
    decode, encode,
    param_type::{ParamType, Reader},
    token::{LenientTokenizer, StrictTokenizer, Token, Tokenizer},
    Contract, Event, Function, Hash,
};
use itertools::Itertools;
use sha3::{Digest, Keccak256};
use std::fs::File;

use super::Command;
use crate::core::context::Context;

pub fn ethabi_encode_function_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("ethabi-encode-function")
        .about("Load function from JSON ABI file.")
        .arg(Arg::new("abi-path").takes_value(true).required(true))
        .arg(
            Arg::new("function_name_or_signature")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("params")
                .short('p')
                .takes_value(true)
                .number_of_values(1)
                .multiple_occurrences(true)
                .required(true),
        )
        .arg(
            Arg::new("lenient")
                .help("Allow short representation of input params.")
                .short('l')
                .long("lenient"),
        )
        .handler(|_cmd, m, _ctx| {
            let abi_path = m.value_of("abi-path").unwrap();
            let function_name_or_signature = m.value_of("function_name_or_signature").unwrap();
            let params = m
                .values_of("params")
                .unwrap_or_default()
                .map(str::to_string)
                .collect::<Vec<String>>();
            let lenient = m.is_present("lenient");

            let encoded = encode_input(abi_path, function_name_or_signature, &params, lenient)?;
            println!("0x{encoded}");

            Ok(())
        })
}

pub fn ethabi_encode_params_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("ethabi-encode-params")
        .about("Specify types of input params inline.")
        .arg(
            Arg::new("type-or-param")
                .short('v')
                .takes_value(true)
                .number_of_values(2)
                .multiple_occurrences(true)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("lenient")
                .help("Allow short representation of input params (numbers are in decimal form).")
                .short('l')
                .long("lenient"),
        )
        .handler(|_cmd, m, _ctx| {
            let params = m
                .values_of("type-or-param")
                .unwrap_or_default()
                .map(str::to_string)
                .collect::<Vec<String>>();
            let lenient = m.is_present("lenient");

            let encoded = encode_params(&params, lenient)?;
            println!("0x{encoded}");

            Ok(())
        })
}

pub fn ethabi_encode_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("ethabi-encode")
        .about("Encode ABI call.")
        .subcommand_required_else_help(true)
        .subcommands([
            ethabi_encode_function_cmd().name("function"),
            ethabi_encode_params_cmd().name("params"),
        ])
}

pub fn ethabi_decode_function_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("ethabi-decode-function")
        .about("Load function from JSON ABI file.")
        .arg(Arg::new("abi-path").takes_value(true).required(true))
        .arg(
            Arg::new("function_name_or_signature")
                .takes_value(true)
                .required(true),
        )
        .arg(Arg::new("data").takes_value(true).required(true))
        .handler(|_cmd, m, _ctx| {
            let abi_path = m.value_of("abi-path").unwrap();
            let function_name_or_signature = m.value_of("function_name_or_signature").unwrap();
            let data = m.value_of("data").unwrap();

            let decoded = decode_call_output(abi_path, function_name_or_signature, data)?;
            println!("{decoded}");

            Ok(())
        })
}

pub fn ethabi_decode_params_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("ethabi-decode-params")
        .about("Specify types of input params inline.")
        .arg(
            Arg::new("type")
                .short('t')
                .takes_value(true)
                .number_of_values(1)
                .multiple_occurrences(true),
        )
        .arg(Arg::new("data").takes_value(true).required(true))
        .handler(|_cmd, m, _ctx| {
            let types = m
                .values_of("type")
                .unwrap_or_default()
                .map(str::to_string)
                .collect::<Vec<String>>();
            let data = m.value_of("data").unwrap();

            let decoded = decode_params(&types, data)?;
            println!("{decoded}");

            Ok(())
        })
}

pub fn ethabi_decode_log_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("ethabi-decode-log")
        .about("Decode event log.")
        .arg(Arg::new("abi-path").takes_value(true).required(true))
        .arg(
            Arg::new("event-name-or-signature")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("topic")
                .short('l')
                .takes_value(true)
                .number_of_values(1)
                .multiple_occurrences(true),
        )
        .arg(Arg::new("data").takes_value(true).required(true))
        .handler(|_cmd, m, _ctx| {
            let abi_path = m.value_of("abi-path").unwrap();
            let event_name_or_signature = m.value_of("event-name-or-signature").unwrap();
            let topics = m
                .values_of("topic")
                .unwrap_or_default()
                .map(str::to_string)
                .collect::<Vec<String>>();
            let data = m.value_of("data").unwrap();

            let decoded = decode_log(abi_path, event_name_or_signature, &topics, data)?;
            println!("{decoded}");

            Ok(())
        })
}

pub fn ethabi_decode_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("ethabi-encode")
        .about("Decode ABI call result.")
        .subcommand_required_else_help(true)
        .subcommands([
            ethabi_decode_function_cmd().name("function"),
            ethabi_decode_params_cmd().name("params"),
            ethabi_decode_log_cmd().name("log"),
        ])
}

pub fn ethabi_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    let version = "17.0.0";
    let author = concat!(
        "Parity Technologies <admin@parity.io>\n",
        "Artem Vorotnikov <artem@vorotnikov.me>\n",
        "Nicholas Rodrigues Lordello <nlordell@gmail.com>\n",
    );

    Command::<Context<Co, Ex, Ev>>::new("ethabi")
        .version(version)
        .author(author)
        .about("Ethereum ABI coder.")
        .subcommand_required_else_help(true)
        .subcommands([
            ethabi_encode_cmd().name("encode"),
            ethabi_decode_cmd().name("decode"),
        ])
}

// The following code is from `ethabi-cli`.

fn load_function(path: &str, name_or_signature: &str) -> anyhow::Result<Function> {
    let file = File::open(path)?;
    let contract = Contract::load(file)?;
    let params_start = name_or_signature.find('(');

    match params_start {
        // It's a signature
        Some(params_start) => {
            let name = &name_or_signature[..params_start];

            contract
                .functions_by_name(name)?
                .iter()
                .find(|f| f.signature() == name_or_signature)
                .cloned()
                .ok_or_else(|| anyhow!("invalid function signature `{}`", name_or_signature))
        }

        // It's a name
        None => {
            let functions = contract.functions_by_name(name_or_signature)?;
            match functions.len() {
                0 => unreachable!(),
                1 => Ok(functions[0].clone()),
                _ => Err(anyhow!(
                    "More than one function found for name `{}`, try providing the full signature",
                    name_or_signature
                )),
            }
        }
    }
}

fn load_event(path: &str, name_or_signature: &str) -> anyhow::Result<Event> {
    let file = File::open(path)?;
    let contract = Contract::load(file)?;
    let params_start = name_or_signature.find('(');

    match params_start {
        // It's a signature.
        Some(params_start) => {
            let name = &name_or_signature[..params_start];
            let signature = hash_signature(name_or_signature);
            contract
                .events_by_name(name)?
                .iter()
                .find(|event| event.signature() == signature)
                .cloned()
                .ok_or_else(|| anyhow!("Invalid signature `{}`", signature))
        }

        // It's a name.
        None => {
            let events = contract.events_by_name(name_or_signature)?;
            match events.len() {
                0 => unreachable!(),
                1 => Ok(events[0].clone()),
                _ => Err(anyhow!(
                    "More than one function found for name `{}`, try providing the full signature",
                    name_or_signature
                )),
            }
        }
    }
}

fn parse_tokens(params: &[(ParamType, &str)], lenient: bool) -> anyhow::Result<Vec<Token>> {
    params
        .iter()
        .map(|&(ref param, value)| match lenient {
            true => LenientTokenizer::tokenize(param, value),
            false => StrictTokenizer::tokenize(param, value),
        })
        .collect::<Result<_, _>>()
        .map_err(From::from)
}

fn encode_input(
    path: &str,
    name_or_signature: &str,
    values: &[String],
    lenient: bool,
) -> anyhow::Result<String> {
    let function = load_function(path, name_or_signature)?;

    let params: Vec<_> = function
        .inputs
        .iter()
        .map(|param| param.kind.clone())
        .zip(values.iter().map(|v| v as &str))
        .collect();

    let tokens = parse_tokens(&params, lenient)?;
    let result = function.encode_input(&tokens)?;

    Ok(hex::encode(&result))
}

fn encode_params(params: &[String], lenient: bool) -> anyhow::Result<String> {
    assert_eq!(params.len() % 2, 0);

    let params = params
        .iter()
        .tuples::<(_, _)>()
        .map(|(x, y)| Reader::read(x).map(|z| (z, y.as_str())))
        .collect::<Result<Vec<_>, _>>()?;

    let tokens = parse_tokens(params.as_slice(), lenient)?;
    let result = encode(&tokens);

    Ok(hex::encode(&result))
}

fn decode_call_output(path: &str, name_or_signature: &str, data: &str) -> anyhow::Result<String> {
    let function = load_function(path, name_or_signature)?;
    let data: Vec<u8> = hex::decode(&data)?;
    let tokens = function.decode_output(&data)?;
    let types = function.outputs;

    assert_eq!(types.len(), tokens.len());

    let result = types
        .iter()
        .zip(tokens.iter())
        .map(|(ty, to)| format!("{} {}", ty.kind, to))
        .collect::<Vec<String>>()
        .join("\n");

    Ok(result)
}

fn decode_params(types: &[String], data: &str) -> anyhow::Result<String> {
    let types: Vec<ParamType> = types
        .iter()
        .map(|s| Reader::read(s))
        .collect::<Result<_, _>>()?;

    let data: Vec<u8> = hex::decode(&data)?;

    let tokens = decode(&types, &data)?;

    assert_eq!(types.len(), tokens.len());

    let result = types
        .iter()
        .zip(tokens.iter())
        .map(|(ty, to)| format!("{} {}", ty, to))
        .collect::<Vec<String>>()
        .join("\n");

    Ok(result)
}

fn decode_log(
    path: &str,
    name_or_signature: &str,
    topics: &[String],
    data: &str,
) -> anyhow::Result<String> {
    let event = load_event(path, name_or_signature)?;
    let topics: Vec<Hash> = topics.iter().map(|t| t.parse()).collect::<Result<_, _>>()?;
    let data = hex::decode(data)?;
    let decoded = event.parse_log((topics, data).into())?;

    let result = decoded
        .params
        .into_iter()
        .map(|log_param| format!("{} {}", log_param.name, log_param.value))
        .collect::<Vec<String>>()
        .join("\n");

    Ok(result)
}

fn hash_signature(sig: &str) -> Hash {
    Hash::from_slice(Keccak256::digest(sig.replace(' ', "").as_bytes()).as_slice())
}
