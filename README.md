# cloud-cli

The command line interface(CLI) to interact with `CITA-Cloud`.

The primary design goal of this tool is to be simple and easy-use.

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

cloud-cli 0.1.0
The command line interface to interact with `CITA-Cloud`.

USAGE:
    cldi [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -e, --executor_addr <executor_addr>    executor address
    -r, --rpc_addr <rpc_addr>              rpc(controller) address
    -u, --user <user>                      the user(account) to send tx

SUBCOMMANDS:
    account               Manage account
    bench                 Send multiple txs with random content
    block-at              Get block by number
    block-number          Get block number
    call                  Executor call
    completions           Generate completions for current shell
    create                Create contract
    emergency-brake       Send emergency brake cmd to chain
    get-abi               Get specific contract abi
    get-balance           Get balance by account address
    get-code              Get code by contract address
    get-tx                Get transaction by hash
    help                  Prints this message or the help of the given subcommand(s)
    peer-count            Get peer count
    receipt               Get receipt by tx_hash
    send                  Send transaction
    set-block-interval    Set block inteval
    store-abi             Store abi
    system-config         Get system config
    update-admin          Update admin of the chain
    update-validators     Update validators of the chain
```

### Connection 

The `cldi` needs to know how to connect to your chain. You may tell it by `-r` (rpc addr, default to `localhost:50004`) and `-e` (executor addr, default to `localhost:50002`).

Those addrs can also be specified by env `CITA_CLOUD_EXECUTOR_ADDR` and `CITA_CLOUD_CONTROLLER_ADDR`.

### Account

WARNING: all account's private keys are stored unencrypted in `$HOME/.cloud-cli`. Security is not a consideration since this tool is mainly used for dev & test for now.

---

To execute commands that requires sending transactions to the chain, an account(keypair) is needed to sign those transactions.

`cldi` will generate a default account(and called `default`). If not specified otherwise, `cldi` use this default account to send transactions for you.

You can also create a new account.
```bash
$ cldi account create Akari
user: `Akari`
account_addr: 0xf54b1e1f4cc3756bc9e46fd052f4f982168f5084
```

And use this account to send a transaction to the chain.

There are three ways to select an account, `login` is recommanded.
```bash
# by login
$ cldi account login Akari

# by param
$ cldi -u Akari send -t 0xd32c8d890697ef71f494bf7566fe8bbba032909c 0x4f2be91f

# by env
$ export CITA_CLOUD_DEFAULT_USER="Akari"
$ cldi send -u Akari -t 0xd32c8d890697ef71f494bf7566fe8bbba032909c 0x4f2be91f
```

We can exxport this account.
```json
$ cldi account export Akari
{
  "account_addr": "0xf54b1e1f4cc3756bc9e46fd052f4f982168f5084",
  "private_key": "0xb76b61692c1e8a5367fc6aded53d21869c92a10a91fbb4d6f39fcecea7c2e121",
  "public_key": "0x11116f3fb4e47a308568df01ed4cb571930b6a8ada30d09d864536d1812cb69c68d94bc0622b0c484339d0acaf842d2c52b860d24512fb80b0587fac117b5b02"
}
```

Import an account from keypair.

- `-p`(or `--pk`), is the public key.

- `-s`(or `--sk`), is the secret(private) key.

```
$ cldi account import Kyoko \
-p 0xd743ed02f0c80cb153f11aac559c0caa3178cd344207a5027958fbb5dd90d8e330a9170a91bf80e3b054a8df7996222d2ddb1c50ed09ed4064506bde86631f5b \
-s 0xbea6167716036cada29f0680e5d135aa10efb66b007a43790f8f72e2916a3084

OK, account `Kyoko` imported
```

Check accounts.
```json
[
  {
    "addr": "0x3820e159546d1c1359095db659178feb6cb43215",
    "user": "default"
  },
  {
    "addr": "0xfbfe83ef2c4cd7f46f2e8a351692bb9287ada992",
    "user": "Kyoko"
  },
  {
    "addr": "0xf54b1e1f4cc3756bc9e46fd052f4f982168f5084",
    "user": "Akari"
  },
]
```

If you want to know how to use one command exactly, you can type it with `-h` to see the help info.

```
$ cldi send -h
cldi-send
Send transaction

USAGE:
    cldi send [OPTIONS] <data> --to <to>

ARGS:
    <data>    the data of the tx

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -t, --to <to>          the address to send
    -v, --value <value>    the value to send
```

## Examples

Get system config using default addresses.

```json
$ cldi system-config
{
  "admin": "0x329a972ee38e0e7fa66158a7ba1de923ccb6e15a",
  "block_interval": 0,
  "chain_id": "0x26b0b83e7281be3b117658b6f2636d0368cad3d74f22243428f5401a4b70897e",
  "validators": [
    "0x35c274869ec6587da5aeaa9375332184f5d4d4c6",
    "0x42eeaf0097fe9bff9925700cf4a85532c36a10fb",
    "0x0cf75e9d21498d695dc44acc204f743c7289f0d2"
  ],
  "version": 0
}
```

Get block number using specified addrs.
```bash
$ cldi -r localhost:50004 -e localhost:50002 block-number
block_number: 3761
```


Get block by block-number using env.
```json
$ export CITA_CLOUD_RPC_ADDR="localhost:50004"
$ export CITA_CLOUD_EXECUTOR_ADDR="localhost:50002"
$ cldi block-at 3761
{
  "height": 3761,
  "prev_hash": "0x77eff82958a2c54058446a8bac3e01abdadf8b3cea126950a8fdd9c5ffc9fc39",
  "proposer": "0x0cf75e9d21498d695dc44acc204f743c7289f0d2",
  "timestamp": "2021-06-08 16:45:29.811 +08:00",
  "transaction_root": "0x1ab21d8355cfa17f8e61194831e81a8f22bec8c728fefb747ed035eb5082aa2b",
  "tx_count": 0,
  "tx_hashes": [],
  "version": 0
}
```

Create a ethereum contract. It's an example contract that stores a counter which increased by one on every valid tx.
```
$ cldi create 0x608060405234801561001057600080fd5b5060f58061001f6000396000f3006080604052600436106053576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff16806306661abd1460585780634f2be91f146080578063d826f88f146094575b600080fd5b348015606357600080fd5b50606a60a8565b6040518082815260200191505060405180910390f35b348015608b57600080fd5b50609260ae565b005b348015609f57600080fd5b5060a660c0565b005b60005481565b60016000808282540192505081905550565b600080819055505600a165627a7a72305820faa1d1f51d7b5ca2b200e0f6cdef4f2d7e44ee686209e300beb1146f40d32dee0029

tx_hash: 0xeab2a49cce72b3d2a1d88a3ad6b243484ff626afa55d0bb9390fc6e987ae79a9
```

Get the contract's address by inspecting the receipt of the create tx.

```json
$ cldi receipt 0xeab2a49cce72b3d2a1d88a3ad6b243484ff626afa55d0bb9390fc6e987ae79a9

{
  "block_hash": "0xa4cf163d0ded07ee24c98ddd05598e6f4f2e034062d98ba047fb26c30dad1755",
  "block_number": 3796,
  "contract_addr": "0x21a2284ba7d03ca6b57cfc671c0f1a77e21aa925",
  "cumulative_quota_used": "0x0000000000000000000000000000000000000000000000000000000000018ed3",
  "error_msg": "",
  "logs": [],
  "logs_bloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "quota_used": "0x0000000000000000000000000000000000000000000000000000000000018ed3",
  "state_root": "0xbe9518f6c50597e914856fb6275710192429dcdbdcfe08151cd8258180be24b4",
  "tx_hash": "0xeab2a49cce72b3d2a1d88a3ad6b243484ff626afa55d0bb9390fc6e987ae79a9",
  "tx_index": 0
}
```

We can see the contract addr is `0x21a2284ba7d03ca6b57cfc671c0f1a77e21aa925`.

Let's check that its counter has been appropriately initialized to `0`.

```
$ cldi call -t 0x21a2284ba7d03ca6b57cfc671c0f1a77e21aa925 0x06661abd
result: 0x0000000000000000000000000000000000000000000000000000000000000000
```

Send a tx to increase its counter.

```bash
$ cldi send -t 0x21a2284ba7d03ca6b57cfc671c0f1a77e21aa925 0x4f2be91f

tx_hash: 0xe636e13dbe728c9a6d91428eeba296cf9f7bd8dc5ad34b899672b8c2380abf36
```

Wait a few seconds for its execution.

Then check the contract's counter.
```
$ cldi call -t 0x21a2284ba7d03ca6b57cfc671c0f1a77e21aa925 0x06661abd
result: 0x0000000000000000000000000000000000000000000000000000000000000001
```

Now, it's `1` as expected.
