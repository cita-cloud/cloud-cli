# rpc

未分类的rpc命令。

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
    parse-proof  parse consensus proof
    help         Print this message or the help of the given subcommand(s)
```

## add-node

调用controller的`add-node`接口，这个接口曾用于让network微服务连接一个新的节点。

```plaintext
$ cldi rpc add-node -h
cldi-rpc-add-node
call add-node rpc

USAGE:
    cldi rpc add-node <port> <domain>

ARGS:
    <port>      the port of the new node
    <domain>    the domain name of the new node

OPTIONS:
    -h, --help    Print help information
```

## store-abi

通过发送交易，在链上保存合约的`ABI`。

```plaintext
$ cldi rpc store-abi -h
cldi-rpc-store-abi
Store EVM contract ABI

USAGE:
    cldi rpc store-abi [OPTIONS] <addr> <abi>

ARGS:
    <addr>
    <abi>

OPTIONS:
    -q, --quota <quota>                the quota of this tx [default: 1073741824]
        --until <valid-until-block>    this tx is valid until the given block height. `+h` means
                                       `<current-height> + h` [default: +95]
    -h, --help                         Print help information
```

### parse-proof

从字节码解析并打印共识的Proof信息，默认`crypto-type`为`SM`，默认`consensus-type`为`BFT`

```plaintext
$ cldi rpc parse-proof -h
cldi-rpc-parse-proof 
parse consensus proof

USAGE:
    cldi rpc parse-proof [OPTIONS] <proof>

ARGS:
    <proof>    plain proof data with `0x` prefix

OPTIONS:
        --consensus <consensus-type>    The consensus type of the proof. [default:
                                        <current-context-crypto-type>] [possible values: BFT,
                                        OVERLORD]
        --crypto <crypto-type>          The crypto type of the proof. [default:
                                        <current-context-crypto-type>] [possible values: SM, ETH]
    -h, --help                          Print help information
```
