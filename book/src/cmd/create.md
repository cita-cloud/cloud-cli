# create

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
