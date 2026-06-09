# 语言偏好
**始终使用简体中文进行回复、解释和生成文档。**

# 注意事项

- 生成C和Rust spec归约时，总是C或者Rust签名写在最前面

- C spec归约中必须明确注明每个函数/全局符号的**导出状态**：若符号为 musl 内部实现、不对外部用户暴露（如 `__` 前缀的内部函数、`static` 变量等），需标注 `[Visibility]: Internal (不导出)` 说明

- **rusl 项目必须是 `#![no_std]` 实现**，不依赖 Rust 标准库。所有依赖的外部 crate 也必须兼容 `no_std` 环境（如 `bitflags` 等支持 `no_std` 的 crate）

- **rusl 任何时候都禁止使用 std或者其他libc**，测试或者实现，任何时候不允许使用std或者别的libc。

- 非必要时不使用unsafe，在unsafe代码块内部不要再使用unsafe，一般来说单条语句unsafe就只包括它自身不要将大段代码放在unsafe里。

- rusl目前的测试和编译环境已经完全配置好了，不要修改.cargo/config.toml或者Cargo.toml，只能修改源码

- rusl集成测试是对于musl libc中的对外api进行的测试，也可以测试rusl内部实现，rusl内部的辅助和中间函数应该放在src下的单元测试中。

- 集成测试应当保证musl libc能够通过，若不通过则修改测试代码自身

- 除了rusl-tester agent，所有agent不能修改任何集成测试代码

- rusl-*crate导出的接口皆为safe

- 单元测试和集成测试都使用test!宏，不能使用#[test]

- **musl `__` 前缀符号必须同时导出**：musl 中 `__xxx` 是主实现，`xxx` 是其弱别名（如 `__strchrnul`/`strchrnul`、`__stpcpy`/`stpcpy`、`__memrchr`/`memrchr`）。musl 内部代码直接调用 `__` 版本，rusl 必须同时提供两者。

- **集成测试时应该只测试musl libc对于用户可见的对外导出接口**，禁止测试任何`__`开头的内部符号，这些符号应该放在单元测试中。

- **集成测试中不能使用extern "C"**,而是使用rusl-main的api模块导出接口。

# 测试musl-libc

```
make test-c
```

# 测试rusl的对外api

```
make inte-test
```