# 命令介绍

## get

获取链上数据相关的命令。

```plaintext
$ cldi help get
cldi-get
Get data from chain

USAGE:
    cldi get <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    abi              Get the specific contract ABI
    balance          Get balance by account address
    block            Get block by block height or hash(0x)
    code             Get code by contract address
    tx               Get transaction data by tx_hash
    peer-count       Get peer count
    peers-info       Get peers info
    nonce            Get the nonce of this account
    receipt          Get EVM execution receipt by tx_hash
    version          Get version
    system-config    Get system config
    block-hash       Get block hash by block height
    block-number     Get block number
    help             Print this message or the help of the given subcommand(s)
```

## send

注意和`cita-cli`不同：如果想要创建合约，请使用create命令。

```plaintext
$ cldi help send
cldi-send
Send transaction

USAGE:
    cldi send [OPTIONS] <to> [data]

ARGS:
    <to>      the target address of this tx
    <data>    the data of this tx [default: 0x]

OPTIONS:
    -v, --value <value>                the value of this tx [default: 0x0]
    -q, --quota <quota>                the quota of this tx [default: 3000000]
        --until <valid-until-block>    this tx is valid until the given block height. `+h` means
                                       `<current-height> + h` [default: +95]
    -h, --help                         Print help information
```

## call

```plaintext
$ cldi help call
cldi-call
Call executor

USAGE:
    cldi call [OPTIONS] <to> <data>

ARGS:
    <to>
    <data>

OPTIONS:
    -f, --from <from>    default to use current account address
    -h, --help           Print help information
```

## create

```plaintext
$ cldi help create
cldi-create
create an EVM contract

USAGE:
    cldi create [OPTIONS] <data>

ARGS:
    <data>    the data of this tx

OPTIONS:
    -v, --value <value>                the value of this tx [default: 0x0]
    -q, --quota <quota>                the quota of this tx [default: 3000000]
        --until <valid-until-block>    this tx is valid until the given block height. `+h` means
                                       `<current-height> + h` [default: +95]
    -h, --help                         Print help information
```

## context

context相关命令。

```plaintext
$ cldi help context
cldi-context
Context commands

USAGE:
    cldi context <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    save
    list       list contexts
    delete
    default    set a context as default and switch current context to it
    help       Print this message or the help of the given subcommand(s)
```

## account

account相关命令。

```plaintext
$ cldi help account
cldi-account
Account commands

USAGE:
    cldi account <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    generate    generate a new account
    list        list accounts
    import      import account
    export      export account
    unlock      unlock a account
    lock        lock a account
    help        Print this message or the help of the given subcommand(s)
```
当前使用的账户可以在`context list`展示的的current setting中查看。


## admin

admin相关命令，用于管理链的配置。

```plaintext
$ cldi help admin
cldi-admin
The admin commands for managing chain

USAGE:
    cldi admin <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    update-admin          Update admin of the chain
    update-validators     Update validators of the chain
    set-block-interval    Set block interval
    emergency-brake       Send emergency brake cmd to chain
    help                  Print this message or the help of the given subcommand(s)
```

这些命令必须以链的管理员账号发送，否则链上会返回错误。具体来说，当前账户的地址必须和链配置的管理员地址一致。

## rpc
```plaintext
$ cldi help rpc
cldi-rpc
Other RPC commands

USAGE:
    cldi rpc <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    add-node     call add-node rpc
    store-abi    Store EVM contract ABI
    help         Print this message or the help of the given subcommand(s)
```

## ethabi

这个子命令来自[ethabi](https://github.com/rust-ethereum/ethabi)，请参考官方文档。

```plaintext
$ cldi help ethabi
cldi-ethabi
Ethereum ABI coder.

USAGE:
    cldi ethabi <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    encode    Encode ABI call.
    decode    Decode ABI call result.
    help      Print this message or the help of the given subcommand(s)
```

## bench
```plaintext
$ cldi help bench
cldi-bench
Simple benchmarks

USAGE:
    cldi bench <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    send    Send transactions with {-c} workers over {--connections} connections
    call    Call executor with {-c} workers over {--connections} connections
    help    Print this message or the help of the given subcommand(s)
```

## watch
观察当前链的出块情况。展示的时间是块头的时间戳，并且会以开始观察的第一个块时间戳为起始时间00:00。

```plaintext
$ cldi help watch
cldi-watch
Watch blocks

USAGE:
    cldi watch [OPTIONS]

OPTIONS:
    -b, --begin <begin>
            the block height starts from. You can use +/- prefix to seek from current height

    -e, --end <end>
            the block height ends at. You can use +/- prefix to seek from current height

    -t, --until <until-finalized-txs>
            stop watching when finalized txs reach the given limit

    -h, --help
            Print help information
```

示例
```plaintext
# 从当前块开始观察
cldi> watch
# 从当前块开始观察，直到有100笔交易上链
cldi> watch --until 100
# 从块高0到块高10
cldi> watch --begin 0 --end 10
# 从当前块高-10到当前块高+10
cldi> watch --begin -10 --end +10
```

### help

```plaintext
$ cldi help help
cldi-help
Print this message or the help of the given subcommand(s)

USAGE:
    cldi help [SUBCOMMAND]...

ARGS:
    <SUBCOMMAND>...    The subcommand whose help message to display
```
