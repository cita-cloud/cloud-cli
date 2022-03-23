# completions

命令行模式下的补全。

```plaintext
$ cldi help completions
cldi-completions
Generate completions for current shell. Add the output script to `.profile` or `.bashrc` etc. to
make it effective.

USAGE:
    cldi completions <shell>

ARGS:
    <shell>    [possible values: bash, zsh, powershell, fish, elvish]

OPTIONS:
    -h, --help    Print help information
```

这个命令会输出补全脚本，需要添加到例如`.profile`, `.bashrc`里才能生效。

以bash为例，将下列脚本添加到`.bashrc`里即可。
```bash
source <(cldi completions bash)
```
