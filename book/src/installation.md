# 安装

有几种不同的安装方法。

## 直接下载预编译的二进制
TODO

## 从源码编译

### 1. 安装Rust
如果你没有Rust环境，可以执行以下命令，通过rustup安装。
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 编译并安装cloud-cli

```bash
# TODO
$ cargo install --git https://github.com/cita-cloud/cloud-cli --branch main
```

也可以先把项目clone到本地。

```bash
$ git clone https://github.com/cita-cloud/cloud-cli
$ cd cloud-cli
$ cargo install --path .
```

