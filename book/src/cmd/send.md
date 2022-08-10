# send

发送交易。

注意和`cita-cli`不同，`<to>`是必传参数，如果想要创建合约，请使用create命令。

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
    -q, --quota <quota>                the quota of this tx [default: 200000]
        --until <valid-until-block>    this tx is valid until the given block height. `+h` means
                                       `<current-height> + h` [default: +95]
    -h, --help                         Print help information
```

`valid_until_block`用来限制交易的有效范围，即在多少高度之前可以被打包，可以指定一个确定的高度（例如100），也可以指定为当前高度加多少（例如+95）。

## 示例

```plaintext
cldi> send 0xf064e32407b6cc412fe33f6ba55f578ac413ecdc 0x4f2be91f
```
