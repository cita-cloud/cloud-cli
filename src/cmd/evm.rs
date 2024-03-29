// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cita_cloud_proto::evm::block_number::Lable;
use cita_cloud_proto::evm::BlockNumber;
use clap::Arg;

use crate::crypto::ArrayLike;
use crate::utils::parse_block_number;
use crate::{
    cmd::Command,
    core::{
        context::Context, controller::ControllerBehaviour, evm::EvmBehaviour, evm::EvmBehaviourExt,
    },
    crypto::{Address, Hash},
    display::Display,
    utils::{get_block_height_at, parse_addr, parse_hash, parse_position, Position},
};

pub fn get_receipt<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-receipt")
        .about("Get EVM execution receipt by tx_hash")
        .arg(
            Arg::new("tx_hash")
                .help("Transaction hash")
                .required(true)
                .value_parser(parse_hash),
        )
        .handler(|_cmd, m, ctx| {
            let tx_hash = *m.get_one::<Hash>("tx_hash").unwrap();

            let receipt = ctx.rt.block_on(ctx.evm.get_receipt(tx_hash))??;
            println!("{}", receipt.display());
            Ok(())
        })
}

pub fn get_code<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-code")
        .about("Get code by contract address")
        .arg(
            Arg::new("addr")
                .help("Contract address")
                .required(true)
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("height")
                .help("Block number, tag or block hash")
                .value_parser(parse_block_number),
        )
        .handler(|_cmd, m, ctx| {
            let addr = *m.get_one::<Address>("addr").unwrap();
            let mut block_number = m
                .get_one::<BlockNumber>("height")
                .unwrap_or(&BlockNumber {
                    lable: Some(Lable::Tag("pending".to_string())),
                })
                .clone();
            if let Lable::Hash(hash) = block_number.lable.clone().unwrap() {
                let height = ctx
                    .rt
                    .block_on(
                        ctx.controller
                            .get_height_by_hash(Hash::try_from_slice(&hash).unwrap()),
                    )??
                    .block_number;
                block_number = BlockNumber {
                    lable: Some(Lable::Height(height)),
                };
            };
            let byte_code = ctx.rt.block_on(ctx.evm.get_code(addr, block_number))??;
            println!("{}", byte_code.display());
            Ok(())
        })
}

pub fn get_balance<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-balance")
        .about("Get balance by account address")
        .arg(
            Arg::new("addr")
                .help("Account address, default to current account")
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("height")
                .help("Block number, tag or block hash")
                .value_parser(parse_block_number),
        )
        .handler(|_cmd, m, ctx| {
            let addr = match m.get_one::<Address>("addr") {
                Some(s) => s.to_owned(),
                None => *ctx.current_account()?.address(),
            };
            let mut block_number = m
                .get_one::<BlockNumber>("height")
                .unwrap_or(&BlockNumber {
                    lable: Some(Lable::Tag("pending".to_string())),
                })
                .clone();
            if let Lable::Hash(hash) = block_number.lable.clone().unwrap() {
                let height = ctx
                    .rt
                    .block_on(
                        ctx.controller
                            .get_height_by_hash(Hash::try_from_slice(&hash).unwrap()),
                    )??
                    .block_number;
                block_number = BlockNumber {
                    lable: Some(Lable::Height(height)),
                };
            };

            let balance = ctx.rt.block_on(ctx.evm.get_balance(addr, block_number))??;
            println!("{}", balance.display());
            Ok(())
        })
}

pub fn get_account_nonce<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-account-nonce")
        .about("Get the nonce of this account")
        .arg(
            Arg::new("addr")
                .help("Account address, default to current account")
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("height")
                .help("Block number, tag or block hash")
                .value_parser(parse_block_number),
        )
        .handler(|_cmd, m, ctx| {
            let addr = match m.get_one::<Address>("addr") {
                Some(s) => s.to_owned(),
                None => *ctx.current_account()?.address(),
            };
            let mut block_number = m
                .get_one::<BlockNumber>("height")
                .unwrap_or(&BlockNumber {
                    lable: Some(Lable::Tag("pending".to_string())),
                })
                .clone();
            if let Lable::Hash(hash) = block_number.lable.clone().unwrap() {
                let height = ctx
                    .rt
                    .block_on(
                        ctx.controller
                            .get_height_by_hash(Hash::try_from_slice(&hash).unwrap()),
                    )??
                    .block_number;
                block_number = BlockNumber {
                    lable: Some(Lable::Height(height)),
                };
            };
            let nonce = ctx
                .rt
                .block_on(ctx.evm.get_tx_count(addr, block_number))??;
            println!("{}", nonce.display());
            Ok(())
        })
}

pub fn get_contract_abi<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-contract-abi")
        .about("Get the specific contract ABI")
        .arg(
            Arg::new("addr")
                .help("Contract address")
                .required(true)
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("height")
                .help("Block number, tag or block hash")
                .value_parser(parse_block_number),
        )
        .handler(|_cmd, m, ctx| {
            let addr = *m.get_one::<Address>("addr").unwrap();
            let mut block_number = m
                .get_one::<BlockNumber>("height")
                .unwrap_or(&BlockNumber {
                    lable: Some(Lable::Tag("pending".to_string())),
                })
                .clone();
            if let Lable::Hash(hash) = block_number.lable.clone().unwrap() {
                let height = ctx
                    .rt
                    .block_on(
                        ctx.controller
                            .get_height_by_hash(Hash::try_from_slice(&hash).unwrap()),
                    )??
                    .block_number;
                block_number = BlockNumber {
                    lable: Some(Lable::Height(height)),
                };
            };
            let byte_abi = ctx.rt.block_on(ctx.evm.get_abi(addr, block_number))??;
            println!("{}", byte_abi.display());
            Ok(())
        })
}

pub fn store_abi<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: ControllerBehaviour + Send + Sync,
{
    Command::<Context<Co, Ex, Ev>>::new("store-abi")
        .about("Store EVM contract ABI")
        .arg(
            Arg::new("addr")
                .required(true)
                .value_parser(parse_addr),
        )
        .arg(Arg::new("abi").required(true))
        .arg(
            Arg::new("quota")
                .help("the quota of this tx")
                .short('q')
                .long("quota")
                .default_value("1073741824")
                .value_parser(str::parse::<u64>),
        )
        .arg(
            Arg::new("valid-until-block")
                .help("this tx is valid until the given block height. `+h` means `<current-height> + h`")
                .long("until")
                .default_value("+95")
                .value_parser(parse_position),
        )
        .handler(|_cmd, m, ctx| {
            let tx_hash = ctx.rt.block_on(async {
                let contract_addr = *m.get_one::<Address>("addr").unwrap();
                let abi = m.get_one::<String>("abi").unwrap();
                let quota = *m.get_one::<u64>("quota").unwrap();
                let valid_until_block = {
                    let pos = *m.get_one::<Position>("valid-until-block").unwrap();
                    get_block_height_at(&ctx.controller, pos).await?
                };

                let signer = ctx.current_account()?;
                ctx.controller
                    .store_contract_abi(
                        signer,
                        contract_addr,
                        abi.as_bytes(),
                        quota,
                        valid_until_block,
                    )
                    .await
            })??;
            println!("{}", tx_hash.display());
            Ok(())
        })
}

pub fn get_receipt_proof<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-receipt-proof")
        .about("Get the specific tx_hash's receipt proof")
        .arg(
            Arg::new("tx_hash")
                .help("Input the tx hash to extract receipt proof")
                .required(true)
                .value_parser(parse_hash),
        )
        .handler(|_cmd, m, ctx| {
            let tx_hash = *m.get_one::<Hash>("tx_hash").unwrap();

            let receipt_proof = ctx.rt.block_on(ctx.evm.get_receipt_proof(tx_hash))??;
            println!("{}", receipt_proof.display());
            Ok(())
        })
}

pub fn get_roots_info<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-roots-info")
        .about("Get the specific block's roots info")
        .arg(
            Arg::new("height")
                .help("Block number, tag or block hash")
                .value_parser(parse_block_number),
        )
        .handler(|_cmd, m, ctx| {
            let mut block_number = m
                .get_one::<BlockNumber>("height")
                .unwrap_or(&BlockNumber {
                    lable: Some(Lable::Tag("pending".to_string())),
                })
                .clone();
            if let Lable::Hash(hash) = block_number.lable.clone().unwrap() {
                let height = ctx
                    .rt
                    .block_on(
                        ctx.controller
                            .get_height_by_hash(Hash::try_from_slice(&hash).unwrap()),
                    )??
                    .block_number;
                block_number = BlockNumber {
                    lable: Some(Lable::Height(height)),
                };
            };
            let receipt_info = ctx.rt.block_on(ctx.evm.get_roots_info(block_number))??;
            println!("{}", receipt_info.display());
            Ok(())
        })
}

pub fn get_storage_at<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Ev: EvmBehaviour,
    Co: ControllerBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("get-roots-info")
        .about("Get the specific block's roots info")
        .arg(
            Arg::new("addr")
                .help("Contract address")
                .required(true)
                .value_parser(parse_addr),
        )
        .arg(
            Arg::new("position")
                .help("Input query storage slot position hash")
                .required(true)
                .value_parser(parse_hash),
        )
        .arg(
            Arg::new("height")
                .help("Block number, tag or block hash")
                .value_parser(parse_block_number),
        )
        .handler(|_cmd, m, ctx| {
            let addr = *m.get_one::<Address>("addr").unwrap();
            let position = *m.get_one::<Hash>("position").unwrap();
            let mut block_number = m
                .get_one::<BlockNumber>("height")
                .unwrap_or(&BlockNumber {
                    lable: Some(Lable::Tag("pending".to_string())),
                })
                .clone();
            if let Lable::Hash(hash) = block_number.lable.clone().unwrap() {
                let height = ctx
                    .rt
                    .block_on(
                        ctx.controller
                            .get_height_by_hash(Hash::try_from_slice(&hash).unwrap()),
                    )??
                    .block_number;
                block_number = BlockNumber {
                    lable: Some(Lable::Height(height)),
                };
            };
            let receipt_info =
                ctx.rt
                    .block_on(ctx.evm.get_storage_at(addr, position, block_number))??;
            println!("{}", receipt_info.display());
            Ok(())
        })
}

#[cfg(test)]
mod tests {

    use crate::crypto::Hash;
    use cita_cloud_proto::controller::SystemConfig;
    use cita_cloud_proto::evm::{Balance, ByteAbi, ByteCode, Nonce, Receipt};

    use crate::cmd::cldi_cmd;
    use crate::core::mock::context;

    #[test]
    fn test_evm_subcmds() {
        let cldi_cmd = cldi_cmd();
        let (mut ctx, _temp_dir) = context();

        ctx.evm
            .expect_get_receipt()
            .returning(|_utxo| Ok(Receipt::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "get",
                    "receipt",
                    "0x74ac6372ab461de6817d7146a9b8ad17c35525b13a37f4bb0da325fbfd999f3a",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.evm
            .expect_get_code()
            .returning(|_utxo, _bn| Ok(ByteCode::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "get",
                    "code",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.evm
            .expect_get_balance()
            .returning(|_utxo, _bn| Ok(Balance::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "get",
                    "balance",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.evm
            .expect_get_tx_count()
            .returning(|_utxo, _bn| Ok(Nonce::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "get",
                    "nonce",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.evm
            .expect_get_abi()
            .returning(|_utxo, _bn| Ok(ByteAbi::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "get",
                    "abi",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.controller
            .expect_get_system_config()
            .returning(|| Ok(SystemConfig::default()));

        ctx.controller
            .expect_get_block_number()
            .returning(|_| Ok(100u64));

        ctx.controller
            .expect_send_raw()
            .returning(|_utxo| Ok(Hash::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "rpc",
                    "store-abi",
                    "-q",
                    "200000",
                    "--until",
                    "+80",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "fake abi",
                ],
                &mut ctx,
            )
            .unwrap();

        ctx.evm
            .expect_get_storage_at()
            .returning(|_addr, _position, _bn| Ok(Hash::default()));

        cldi_cmd
            .exec_from(
                [
                    "cldi",
                    "get",
                    "storage-at",
                    "0xf587c2fa24d23175e09d36625cfc447a4b4d679b",
                    "0x0000000000000000000000000000000000000000000000000000000000000000",
                ],
                &mut ctx,
            )
            .unwrap();
    }
}
