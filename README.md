# cloud-cli

The command line interface to interact with `CITA-CLOUD`.

It's in a early stage and under actively development.


## Install

install & update
```sh
cargo install --git https://github.com/cita-cloud/cloud-cli --branch main
```

If you don't have `cargo` yet, you can install it using `rustup`.
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

You can also clone this repo then install it in local.
```
git clone https://github.com/cita-cloud/cloud-cli
cargo install --path cloud-cli
```

## Usage

```
$ cldi help
cloud-cli

USAGE:
    cldi [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -e, --executor_addr <executor_addr>
    -r, --rpc_addr <rpc_addr>

SUBCOMMANDS:
    bench            Send multiple txs with random content
    block_at         Get block by number
    block_number     Get block number
    call             Executor call
    create           Create contract
    get_balance      Get balance by account address
    get_code         Get code by contract address
    get_tx           Get transaction by hash
    help             Prints this message or the help of the given subcommand(s)
    peer_count       Get peer count
    receipt          Get receipt by tx_hash
    send             Send transaction
    system_config    Get system config
```

The cli needs to know how to connect to your chain. You may tell it by `-e`(executor addr, default to `localhost:50002`) and `-r` (rpc addr, default to `localhost:50004`).

Those addrs can also be specified by env `CITA_CLOUD_EXECUTOR_ADDR` and `CITA_CLOUD_CONTROLLER_ADDR`.

## Examples

Get system config using default addrs.

```json
$ cldi system_config
{
  "admin": "0xb24565086e18860d1f86ca99894927eda6470ef5",
  "block_interval": 6,
  "chain_id": "0x26b0b83e7281be3b117658b6f2636d0368cad3d74f22243428f5401a4b70897e",
  "validators": [
    "0x0c2f63a6c14e7ceaf26621c3f1deb4abad69bf67",
    "0x7eca4d1e3bf9b397e79c77c80592a975f7e39075",
    "0x0ac37f9f04d875c8d84bbe7272ca1adadfb74e19"
  ],
  "version": 0
}
```

Get block number using specified addrs.
```
$ cldi -r localhost:50004 -e localhost:50002 block_number
block_number: 207103
```

Get block by block number using env.
```json
$ CITA_CLOUD_CONTROLLER_ADDR="localhost:50004" CITA_CLOUD_EXECUTOR_ADDR="localhost:50002" cldi block_at 207103
{
  "hash": "0x8bef470637320b74bc5cd0223d01916e6df348d1aefc1b38827e82c13d838dab",
  "height": 207103,
  "prev_hash": "0x30e6e9eafcf09f8abff35b85c0b60aaa1c8ec865696fa2e63060511271c15787",
  "proposer": "0x0c2f63a6c14e7ceaf26621c3f1deb4abad69bf67",
  "timestamp": "2021-05-31 10:55:23.453 +08:00",
  "transaction_root": "0x1ab21d8355cfa17f8e61194831e81a8f22bec8c728fefb747ed035eb5082aa2b",
  "tx_count": 0,
  "version": 0
}
```

Create contract. It's an example contract that stores a counter which increased by one on every valid tx.
```
$ cldi create 0x608060405234801561001057600080fd5b5060f58061001f6000396000f3006080604052600436106053576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff16806306661abd1460585780634f2be91f146080578063d826f88f146094575b600080fd5b348015606357600080fd5b50606a60a8565b6040518082815260200191505060405180910390f35b348015608b57600080fd5b50609260ae565b005b348015609f57600080fd5b5060a660c0565b005b60005481565b60016000808282540192505081905550565b600080819055505600a165627a7a72305820faa1d1f51d7b5ca2b200e0f6cdef4f2d7e44ee686209e300beb1146f40d32dee0029

tx_hash: 0x51922e0f67e482b0f629c88f5f8ce6d56dad52633318a40abddebec7634436ae
```

Get the contract's addr by inspecting the receipt of the create tx.

```json
$ cldi receipt 0x51922e0f67e482b0f629c88f5f8ce6d56dad52633318a40abddebec7634436ae
{
  "block_hash": "0xc5d9ea784714fb5b899f81654e69b4be377b2d080b7efc2a3545ee6f0a91d97a",
  "block_number": 207114,
  "contract_addr": "0xd32c8d890697ef71f494bf7566fe8bbba032909c",
  "cumulative_quota_used": "0x0000000000000000000000000000000000000000000000000000000000018ed3",
  "error_msg": "",
  "logs": [],
  "logs_bloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "quota_used": "0x0000000000000000000000000000000000000000000000000000000000018ed3",
  "state_root": "0x060cea462ef403beb301b269390d9094d219cfcb33a9e2a7c59049240759bfc7",
  "tx_hash": "0x51922e0f67e482b0f629c88f5f8ce6d56dad52633318a40abddebec7634436ae",
  "tx_index": "0x060cea462ef403beb301b269390d9094d219cfcb33a9e2a7c59049240759bfc7"
}
```

We can see the contract addr is `0xd32c8d890697ef71f494bf7566fe8bbba032909c`.

Let's check that its counter has been appropriately initialized to `0`.

```
$ cldi call -t 0xd32c8d890697ef71f494bf7566fe8bbba032909c -d 0x06661abd
result: 0x0000000000000000000000000000000000000000000000000000000000000000
```

Send a tx to increase its counter.

```
$ cldi send -t 0xd32c8d890697ef71f494bf7566fe8bbba032909c -d 0x4f2be91f
tx_hash: 0xc75a4556b9503de6e8191378331950467118e4155250349f5ae7a55776d922e1
```

Wait a few seconds for this tx to be executed.

Then check the contract's counter.
```
$ cldi call -t 0xd32c8d890697ef71f494bf7566fe8bbba032909c -d 0x06661abd

result: 0x0000000000000000000000000000000000000000000000000000000000000001
```

Now, it's `1` as expected.
