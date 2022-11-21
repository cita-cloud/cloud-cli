# get

获取链上数据相关的命令。

```plaintext
$ cldi help get
Get data from chain

Usage: cldi get <COMMAND>

Commands:
  abi            Get the specific contract ABI
  balance        Get balance by account address
  block          Get block by block height or hash(0x)
  code           Get code by contract address
  tx             Get transaction data by tx_hash
  nonce          Get the nonce of this account
  receipt        Get EVM execution receipt by tx_hash
  system-config  Get system config
  block-hash     Get block hash by block height
  block-number   Get block number
  node-status    Get node status
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help information
```

## blcok-number

获取节点当前已确认的块高。
```bash
cldi> get block-number
cldi> g bn
```

获取尚未确认的块高。
```bash
cldi> get block-number -p
```

## receipt

获取EVM执行后的交易回执，注意它和get-tx不同，get-tx获取的是交易数据。
```bash
cldi> get receipt 0x8efa5acafdb1a48de23231444d7f28c64d22ebe17a5889a08aeeb3bdd7303197
cldi> g r 0x8efa5acafdb1a48de23231444d7f28c64d22ebe17a5889a08aeeb3bdd7303197
```
