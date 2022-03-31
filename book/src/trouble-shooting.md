# 错误排查

TODO: 欢迎贡献文档

## 编译失败，提示feature unstable

rust版本不够新，更新即可。

```plaintext
$ rustup update
```

## Connection refused

和链建立连接被拒绝，请检查当前环境配置的controller_addr和executor_addr。

```plaintext
$ cldi ctx ls
```

## Admin Check Error

admin命令需要使用管理员账户，管理员账户是在启链时设置的。


## No get receipt

这个是`executor_evm`返回的报错，一般有两种情况：
- 交易没有上链
- 试图获取admin命令发送的UTXO交易的回执。UTXO交易在`executor_evm`里不处理，没有回执。

## Account locked

当前账户带密码，使用`-p`指定密码。
