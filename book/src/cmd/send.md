# send

发送交易。

注意和`cita-cli`不同，<to>是必传参数，如果想要创建合约，请使用create命令。

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
