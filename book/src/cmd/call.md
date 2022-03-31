# call

调用executor提供的call。

这里用到的`<data>`是经过ethabi编码的数据。

```plaintext
$ cldi help call
cldi-call
Call executor

USAGE:
    cldi call [OPTIONS] <to> <data>

ARGS:
    <to>      the target contract address
    <data>    the data of this call request

OPTIONS:
    -f, --from <from>    default to use current account address
    -h, --help           Print help information
```

## 示例

```plaintext
cldi> call 0xf064e32407b6cc412fe33f6ba55f578ac413ecdc 0x06661abd
```
