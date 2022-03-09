# cloud-cli

cloud-cli(简称cldi)是CITA-Cloud命令行工具。它封装了CITA-Cloud构建的链提供的gRPC接口，并提供了一些辅助功能，方便用户与链进行交互。


```plaintext
$ cldi help
cldi
The command line interface to interact with `CITA-Cloud v6.3.0`

USAGE:
    cldi [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -c, --context <context>     context setting
    -e <executor-addr>          executor address
    -h, --help                  Print help information
    -r <controller-addr>        controller address
    -u <account-id>             account id

SUBCOMMANDS:
    account        Key commands
    admin          The admin commands for managing chain
    bench-call     Call executor with {-c} workers over {--connections} connections
    bench-send     Send transactions with {-c} workers over {--connections} connections
    call           Call executor
    completions    Generate completions for current shell. Add the output script to `.profile`
                       or `.bashrc` etc. to make it effective.
    context        context commands
    create         Create EVM contract
    ethabi         Ethereum ABI coder.
    evm            EVM commands
    get            Get chain info
    help           Print this message or the help of the given subcommand(s)
    rpc            RPC commands
    send           Send transaction
    watch          watch
```
