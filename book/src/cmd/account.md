# account

account相关命令。

**警告**：设置了密码的账户在硬盘上是经过加密后存储的，虽然加密算法本身可靠，但在加密算法之外可能存在其它安全漏洞（例如`account delete`没有安全覆写原明文私钥），代码未经安全审计，作者亦非安全专家，不能保证安全性，使用者风险自负(Use at your own risk)。

```plaintext
$ cldi help account
cldi-account
Account commands

USAGE:
    cldi account <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    generate    generate a new account
    list        list accounts
    import      import account
    export      export account
    unlock      unlock account in keystore
    lock        lock account in keystore
    delete      delete account
    help        Print this message or the help of the given subcommand(s)
```
当前使用的账户可以在`context list`展示的的current setting中查看。

如果需要安全地删除账户，可以在`$HOME/.cloud-cli/accounts`下找到账户名对应的toml文件，使用带有防止恢复功能的删除工具删除（粉碎文件）。

带密码的账户需要在cldi命令下传入`-p <password>`解锁，用法与`-c`, `-r`等命令相似。

## 带密码的账号

创建一个带密码的账户root，这个账户root在当前交互会话下可用，无需再输入密码
```plaintext
$ cldi
cldi> account generate --name root --password root
{
  "crypto_type": "SM",
  "address": "0xb293c14d8fc8ff4b24db3926118388b562593d99",
  "public_key": "0x411b418b10005ec32aacf6412a097e87680b29795f723844f6c90e2e850b9d618640ad0fa3011dd67bb667f31656476eb13fdef63329c4756ebd44d3ca265c08",
  "encrypted_sk": "0x36aa2663fa3c2626d7acf051f6a49a81280d7422b49551bcaf9c187d14af2b0d"
}
```
按CTRL-D退出交互模式，重新进入。注意这里可以在进入时直接选中这个账号root
```plaintext
$ cldi -u root
cldi>
```
尝试在不输入密码的情况下使用这个账号，可以看到报错
```plaintext
cldi> send 0xb293c14d8fc8ff4b24db3926118388b562593d99
cannot get current account `root`

Caused by:
    account is locked, please unlock it first(e.g. `cldi -p <password> [subcommand]`)
```
输入密码，与其它命令相似，不带子命令是为当前会话解锁账户，带了子命令则只针对这条子命令。
```plaintext
cldi> -p root
cldi> send 0xb293c14d8fc8ff4b24db3926118388b562593d99
```

