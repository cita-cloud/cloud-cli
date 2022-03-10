# 快速入门

## 配置
### 1. controller和executor的地址

想要与链交互，首先要知道如何访问链。

CITA-Cloud有两个rpc地址，分别是controller和executor微服务。

假设controller的地址为"localhost:50004", executor的地址为"localhost:50002"。

那么我们可以通过`-r`和`-e`来告诉cldi如何访问链：
```bash
# 注意-r和-e必须在子命令之前
$ cldi -r localhost:50004 -e localhost:50002 get block-number
```

### 2. 账户名称

发送交易的命令需要对交易进行签名，我们需要指定签名所使用的账户。
cldi在第一次使用的时候会创建一个名为`default`的默认账户，用户可以通过`-u`来指定账户：
```bash
# 同样地，-u必须在子命令之前
$ cldi -u Alice send --to <to> --value <value> --data <data>
```
创建和导入账户相关的命令请参见TODO。

## 使用Context管理配置

每次都指定微服务的访问地址和使用的账户名称不太方便，我们可以通过context命令来管理这些配置。

```bash
# 创建一个Context
$ cldi -r localhost:50004 -e localhost:50002 -u Alice context save Wonderland
# 将这个Context设为默认
$ cldi context default Wonderland
# 也可以使用-c来切换Context，-c也必须在子命令之前
$ cldi -c Wonderland get block-number
# 列出当前Context信息和已保存的Context
$ cldi context list
```

## 交互模式

cldi提供了命令行模式和交互模式，在未传入子命令的时候cldi会进入交互模式。

交互模式与命令行模式的命令是等价的，例如：
```bash
$ cldi get block-number
```
等价于
```bash
$ cldi
cldi> get block-number
```
在交互模式下，用户可以通过`-c`, `-r` `-e`来改变Context配置。

```bash
# 修改当前全局配置
cldi> -r localhost:50004
# 仅针对这条命令应用这个配置
cldi> -r localhost:50004 get block-number
```

## 简写

cldi提供了很多命令的简写和别名，这里列举一些：
```plaintext
cldi> get block-number
cldi> get bn

cldi> get system-config
cldi> get sc

cldi> context list
cldi> ctx ls
cldi> ctx l
```
