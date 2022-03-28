# bench

简单的性能测试工具。

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

目前支持测试发送交易(send)和executor调用(call)。

## 通用参数

```plaintext
ARGS:
    <total>    Number of tasks in the benchmark [default: 10000]

OPTIONS:
    -c, --concurrency <concurrency>    Number of request workers to run concurrently. Workers will
                                       be distributed evenly among all the connections. [default:
                                       the same as total]
        --connections <connections>    Number of connections connects to server [default: 1]
        --timeout <timeout>            Timeout for each request (in seconds). 0 means no timeout
                                       [default: 0]
```

- 位置参数`<total>`代表总共发多少个请求，默认请求数为10000。
- `-c`或`--concurrency`用来指定并发数，代表同一时刻最多有多少请求在并发进行，默认是所有请求都是并发发出的。
- `--connections`是使用的连接数，`cldi`发起的gRPC请求是会在同一条连接上多路复用的，增加连接数在一定范围内能提高发送速度，默认连接数为1。
- `--timeout`是请求的超时时间，单位是秒，默认为0，即不设置超时。

## bench-send

```plaintext
cldi> bench send -h
cldi-bench-send
Send transactions with {-c} workers over {--connections} connections

USAGE:
    cldi bench send [OPTIONS] [total]

ARGS:
    <total>    Number of tasks in the benchmark [default: 10000]

OPTIONS:
    -c, --concurrency <concurrency>    Number of request workers to run concurrently. Workers will
                                       be distributed evenly among all the connections. [default:
                                       the same as total]
        --connections <connections>    Number of connections connects to server [default: 1]
        --timeout <timeout>            Timeout for each request (in seconds). 0 means no timeout
                                       [default: 0]
    -t, --to <to>                      the target address of this tx. Default to random
    -d, --data <data>                  the data of this tx. Default to random 32 bytes
    -v, --value <value>                the value of this tx [default: 0x0]
    -q, --quota <quota>                the quota of this tx [default: 3000000]
        --until <valid-until-block>    this tx is valid until the given block height. `+h` means
                                       `<current-height> + h` [default: +95]
        --disable-watch                don't watch blocks
    -h, --help                         Print help information
```
