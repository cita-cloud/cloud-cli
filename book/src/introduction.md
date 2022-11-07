# cloud-cli

`cloud-cli`（简称`cldi`）是CITA-Cloud命令行工具。它封装了CITA-Cloud构建的链提供的gRPC接口，并提供了一些辅助功能，方便用户与链进行交互。

```plaintext
$ cldi help
The command line interface to interact with CITA-Cloud

Usage: cldi [OPTIONS] [COMMAND]

Commands:
  get          Get data from chain
  send         Send transaction
  call         Call executor
  create       create an EVM contract
  context      Context commands
  account      Account commands
  admin        The admin commands for managing chain
  rpc          Other RPC commands
  ethabi       Ethereum ABI coder.
  bench        Simple benchmarks
  watch        Watch blocks
  completions  Generate completions for current shell. Add the output script to `.profile` or `.bashrc` etc. to make it effective.
  help         Print this message or the help of the given subcommand(s)

Options:
  -c, --context <context>           context setting
  -r <controller-addr>              controller address
  -e <executor-addr>                executor address
  -u <account-name>                 account name
  -p <password>                     password to unlock the account
      --crypto <crypto-type>        The crypto type of the target chain [possible values: SM, ETH]
      --consensus <consensus-type>  The consensus type of the target chain [possible values: BFT, OVERLORD, RAFT]
  -h, --help                        Print help information
  -V, --version                     Print version information
```
