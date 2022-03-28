# 开发指南

参考`cloud-cli/src/cmd`下的命令实现。

需要异步的话可以从Context里拿到runtime，用这个runtime去block_on即可执行异步任务。

需要client的什么功能，就在Context相应的泛型里加上相应的trait约束。
如果觉得泛型太复杂，也可以把它去掉，把所有泛型都用具体的类型替换掉就可以。

TODO: 有生之年

## TODO

- 把错误处理做好一点，现在用anyhow糊不太好，建议用thiserror做一些具体的类型，然后上层可以做一些判断，打印更有帮助的错误信息。
- 交互模式下的补全，考虑到clap可以构造一些很复杂的命令，想要做对会比较麻烦。好消息是补全本身的实现有rustyline。
- mock测试。
- 给help加个h缩写，可能要绕一下，参考completions的写法。
