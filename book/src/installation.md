# 安装

有几种不同的安装方法。

## 直接下载预编译的二进制
TODO

## 从源码编译

### 1.安装Rust
如果你没有Rust环境，可以执行以下命令，通过rustup安装。
不建议通过其它方式（如apt-get）安装Rust
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2.克隆项目到本地

```
git clone https://github.com/cita-cloud/cloud-cli
cd cloud-cli
```

这时候可以通过`git checkout <branch>`切换到想要使用的分支。

### 3.编译并安装cloud-cli

```
cargo install --path .
```

TODO: 添加换源说明
