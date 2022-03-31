# watch

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
# 使用缩写
cldi> w --begin -10 --end +10 --until 100
```
