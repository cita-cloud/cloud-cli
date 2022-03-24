# cloud-cli
[![CI](https://github.com/whfuyn/cloud-cli/actions/workflows/ci.yaml/badge.svg)](https://github.com/whfuyn/cloud-cli/actions/workflows/ci.yaml)
[![Book CI](https://github.com/whfuyn/cloud-cli/actions/workflows/book-ci.yaml/badge.svg)](https://github.com/whfuyn/cloud-cli/actions/workflows/book-ci.yaml)
[![Security Audit](https://github.com/whfuyn/cloud-cli/actions/workflows/audit.yaml/badge.svg)](https://github.com/whfuyn/cloud-cli/actions/workflows/audit.yaml)

`CITA-Cloud`命令行工具。

## 安装

有几种不同的安装方法。

### 直接下载预编译的二进制文件

cldi有预编译好的二进制可执行文件，可以根据使用环境下载对应的文件。
https://github.com/whfuyn/cloud-cli/releases

如果你不知道如何选择，那么一般来说:
- cldi-x86_64-pc-windows-msvc.zip，如果你是在Windows下
- cldi-x86_64-unknown-linux-gnu.tar.gz，如果你是在Linux下
- cldi-x86_64-apple-darwin.tar.gz，如果你是在MacOS下（非M1）
- cldi-aarch64-apple-darwin.tar.gz，如果你是在MacOS下（M1）

如果出现libc相关问题，可以使用musl版。

如果在ARM上，使用aarch64版。

### 从源码编译

#### 1. 安装Rust
如果你没有Rust环境，可以执行以下命令，通过rustup安装。
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### 2. 编译并安装cloud-cli

更新Rust版本。
```bash
# cloud-cli requires rust 1.58 or above
$ rustup update
```

可以通过通过GitHub安装。

```bash
$ cargo install --git https://github.com/whfuyn/cloud-cli --branch main
```

也可以先把项目clone到本地。

```bash
$ git clone https://github.com/cita-cloud/cloud-cli
$ cargo install --path cloud-cli
```

### 验证安装
```plaintext
$ cldi --version
cldi 0.4.0
```

## 快速入门

### 配置
#### 1. controller和executor的地址

想要与链交互，首先要知道如何访问链。

CITA-Cloud有两个rpc地址，分别是controller和executor微服务。

假设controller的地址为"localhost:50004", executor的地址为"localhost:50002"。

那么我们可以通过`-r`和`-e`来告诉cldi如何访问链：
```bash
# 注意-r和-e必须在子命令之前
$ cldi -r localhost:50004 -e localhost:50002 get block-number
```

#### 2. 账户名称

发送交易的命令需要对交易进行签名，我们需要指定签名所使用的账户。
cldi在第一次使用的时候会创建一个名为`default`的默认账户，用户可以通过`-u`来指定账户：
```bash
# 同样地，-u必须在子命令之前
$ cldi -u Alice send --to <to> --value <value> --data <data>
```
创建和导入账户相关的命令请参见TODO。

### 使用Context管理配置

每次都指定微服务的访问地址和使用的账户名称不太方便，我们可以通过context命令来管理这些配置。

```bash
# 创建一个Context
$ cldi -r localhost:50004 -e localhost:50002 -u Alice context save Wonderland
# 将这个Context设置成默认
$ cldi context default Wonderland
# 也可以使用-c来为当前命令切换Context，-c也必须在子命令之前
$ cldi -c Wonderland get block-number
# 列出当前Context信息和已保存的Context
$ cldi context list
```

### 交互模式

cldi提供了命令行模式和交互模式，在未传入子命令的时候cldi会进入交互模式。

```plaintext
$ cldi
cldi>
```

交互模式与命令行模式的命令是等价的，例如：
```bash
$ cldi get block-number
```
等价于
```bash
$ cldi
cldi> get block-number
```
在交互模式下，用户可以通过`-c`, `-r` `-e`来改变当前会话的Context配置。

```bash
# 修改当前会话配置
cldi> -r localhost:50004
# 仅针对这条命令应用这个配置
cldi> -r localhost:50004 get block-number
```

Q: How to quit cldi?<br>
<del>A: :q</del><br>
A: CTRL-D

### 缩写
<del>effective-cldi</del>

cldi提供了很多命令的缩写，这里列举一些：
```plaintext
cldi> get block-number
cldi> get bn
cldi> g bn

cldi> get system-config
cldi> get sc
cldi> ge sc

cldi> context list
cldi> ctx ls
cldi> ctx l

cldi> bench send
cldi> b send

cldi> watch
cldi> w
```

这些缩写仅为方便用户操作，不作稳定性保证，不建议在脚本中使用。

### 命令行模式下的补全

`cldi completions <shell-name>`命令会输出补全脚本，需要添加到, 例如`.profile`, `.bashrc`里才能生效。

以bash为例，将下列脚本添加到`.bashrc`里即可。
```bash
source <(cldi completions bash)
```
