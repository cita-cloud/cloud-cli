# account

account相关命令。

**警告**：设置了密码的账户在硬盘上是经过加密存储的，虽然加密算法本身可靠，但在加密算法之外可能存在其它安全漏洞（例如`account lock`没有安全擦除原明文私钥），代码未经安全审计，不能保证严格的安全性。

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
    unlock      unlock a account
    lock        lock a account
    help        Print this message or the help of the given subcommand(s)
```
当前使用的账户可以在`context list`展示的的current setting中查看。
