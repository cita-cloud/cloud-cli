# 安装

有几种不同的安装方法。

## 直接下载预编译的二进制

cldi有预编译好的二进制，可以根据使用环境下载对应的文件。
https://github.com/whfuyn/cloud-cli/releases

如果你不知道如何选择，那么一般来说:
- cldi-x86_64-pc-windows-msvc.zip，如果你是在Windows下
- cldi-x86_64-unknown-linux-gnu.tar.gz，如果你是在Linux系统下
- cldi-x86_64-apple-darwin.tar.gz，如果你是在MacOS下（非M1）
- cldi-aarch64-apple-darwin.tar.gz，如果你是在MacOS下（M1）

如果出现libc相关问题，可以使用musl版。
如果在ARM上，使用aarch64版。

## 从源码编译

### 1. 安装Rust
如果你没有Rust环境，可以执行以下命令，通过rustup安装。
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 编译并安装cloud-cli

```bash
# TODO
$ cargo install --git https://github.com/whfuyn/cloud-cli --branch main
```

也可以先把项目clone到本地。

```bash
$ git clone https://github.com/cita-cloud/cloud-cli
$ cd cloud-cli
$ cargo install --path .
```

## 验证安装
```plaintext
$ cldi help
cldi
The command line interface to interact with `CITA-Cloud v6.3.0`

USAGE:
    cldi [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -c, --context <context>     context setting
    -e <executor-addr>          executor address
    -h, --help                  Print help information
    -r <controller-addr>        controller address
    -u <account-name>           account name

SUBCOMMANDS:
    account        Key commands
    admin          The admin commands for managing chain
    bench-call     Call executor with {-c} workers over {--connections} connections
    bench-send     Send transactions with {-c} workers over {--connections} connections
    call           Call executor
    completions    Generate completions for current shell. Add the output script to `.profile`
                       or `.bashrc` etc. to make it effective.
    context        context commands
    create         Create EVM contract
    ethabi         Ethereum ABI coder.
    evm            EVM commands
    get            Get chain info
    help           Print this message or the help of the given subcommand(s)
    rpc            RPC commands
    send           Send transaction
    watch          watch
```
