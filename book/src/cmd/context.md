# context

context相关命令。

```plaintext
$ cldi help context
cldi-context
Context commands

USAGE:
    cldi context <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    save       save context
    list       list contexts
    delete     delete context
    default    set a context as default and switch current context to it
    help       Print this message or the help of the given subcommand(s)
```

## context list

列出环境配置信息。

```plaintext
cldi> context list
cldi> ctx ls
cldi> ctx l
```
```json
{
  // 当前保存的所有环境配置
  "contexts": {
    // 环境配置的名字
    "default": {
       // 这个配置所用的账户名
      "account_name": "default",
       // controller地址
      "controller_addr": "localhost:50004",
       // 使用的密码学算法集
      "crypto_type": "SM",
       // executor地址
      "executor_addr": "localhost:50002"
    },
  },
  // 当前会话的环境配置
  "current_context": {
    "account_name": "default",
    "controller_addr": "localhost:50004",
    "crypto_type": "SM",
    "executor_addr": "localhost:50004"
  },
  // 启动时默认使用的环境配置的名字
  "default_context": "default"
}
```

## context save

保存当前会话的环境配置。

```plaintext
cldi context save <context-name>
```

### 示例

将当前环境配置保存成一个名为new的环境配置。
```plaintext
cldi> context save new
```

以test这个环境配置为基础，controller地址localhost:50004，executor地址localhost:50002，账号名为admin，保存一个名为admin的环境配置。
```plaintext
cldi> -c test -r localhost:50004 -e localhost:50002 -u admin context save admin
```
