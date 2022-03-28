# create

发送交易创建EVM上的合约。合约地址可以在交易的receipt里的contract_addr中查看。

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

## 示例

```plaintext
cldi> create 0x608060405234801561001057600080fd5b5060f58061001f6000396000f3006080604052600436106053576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff16806306661abd1460585780634f2be91f146080578063d826f88f146094575b600080fd5b348015606357600080fd5b50606a60a8565b6040518082815260200191505060405180910390f35b348015608b57600080fd5b50609260ae565b005b348015609f57600080fd5b5060a660c0565b005b60005481565b60016000808282540192505081905550565b600080819055505600a165627a7a72305820faa1d1f51d7b5ca2b200e0f6cdef4f2d7e44ee686209e300beb1146f40d32dee0029
``
