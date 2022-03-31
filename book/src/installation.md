# 安装

有几种不同的安装方法。

## 直接下载预编译的二进制文件

cldi有预编译好的二进制可执行文件，可以根据使用环境下载对应的文件。
https://github.com/whfuyn/cloud-cli/releases

如果你不知道如何选择，那么一般来说:
- cldi-x86_64-pc-windows-msvc.zip，如果你是在Windows下
- cldi-x86_64-unknown-linux-gnu.tar.gz，如果你是在Linux下
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

更新Rust版本。
```bash
# cloud-cli requires rust 1.58 or above
$ rustup update
```

可以通过GitHub安装。

```bash
$ cargo install --git https://github.com/whfuyn/cloud-cli --branch main
```

也可以先把项目clone到本地。

```bash
$ git clone https://github.com/cita-cloud/cloud-cli
$ cargo install --path cloud-cli
```

## 验证安装
```plaintext
$ cldi --version
cldi 0.4.0
```
