# version 规约 (Rust)

> **来源文件**: `musl/src/internal/version.c`
> **目标模块**: `rusl/src/internal/version.rs`
> **复杂度层级**: Level 1 — 纯常量定义

---

## 概述

本模块提供 libc 自身的版本字符串，用于运行时标识、兼容性检查和调试/日志用途。

---

## [RELY]

```
无外部依赖。（版本字符串在编译时由构建系统注入。）
```

## [GUARANTEE]

```
Exported Interface:
  static __libc_version: &[u8];  // NUL 结尾的版本字符串
                                  // [Visibility]: Internal (不导出)
                                  // 供 musl 内部使用（如启动代码可通过 auxv 传递版本信息）
```

---

## 构建时版本注入

Rust 侧版本字符串的来源方式（替代 C 的 `VERSION` 宏）：

1. **使用 `env!("CARGO_PKG_VERSION")`** — 从 `Cargo.toml` 获取版本号
2. **使用 `env!("RUSL_VERSION")`** — 通过构建脚本 (`build.rs`) 在编译时设置环境变量，内容为 `git describe` 输出
3. **使用 `include_str!()`** — 读取构建时生成的版本文件

推荐方案：通过 `build.rs` 调用 `git describe --tags --dirty` 生成版本字符串，以环境变量形式传递给编译器。

---

## `__libc_version` — 内部全局常量

### 签名

```rust
/// musl 内部版本字符串（NUL 结尾）
///
/// 内容在编译时确定，位于只读数据段，进程生命周期内不变。
#[no_mangle]
pub static __libc_version: &[u8] = ...;
```

`[Visibility]: Internal (不导出) — 声明为 hidden，供内部使用`

---

### 声明位置

```rust
// 在 libc 模块中声明:
// pub(crate) static __libc_version: &[u8];
```

---

### 意图 (Intent)

提供 libc 自身的版本字符串，用于：
1. **运行时标识**: 在程序启动时，musl 可通过 `__init_libc` 将版本信息放入辅助向量或调试结构中
2. **兼容性检查**: 理论上可被用于运行时检测 libc 版本（尽管 musl 不鼓励此类做法）
3. **调试/日志**: crash reporter 或调试器可读取此符号获取 libc 版本信息

---

### 前置条件

无。这是一个编译时常量/只读全局变量，程序启动前即已确定其内容。

---

### 后置条件

- `__libc_version` 指向一个 NUL 结尾的字节序列，内容等于构建时确定的版本字符串
- 该字符串位于只读数据段（`.rodata`），不可被修改
- 在进程整个生命周期内保持不变

---

### 不变量

`__libc_version` 的内容在程序启动后永不变更。它的地址（`&__libc_version`）可被安全地跨模块引用。

---

### Rust 实现注意事项

在 Rust `#![no_std]` 实现中：
- 版本号来源：通过 `build.rs` 在编译时运行 `git describe` 并将结果写入环境变量，再通过 `env!()` 宏注入
- 对应 Rust 定义：
  ```rust
  #[no_mangle]
  #[used]
  pub static __libc_version: &[u8] = concat!(env!("RUSL_VERSION"), "\0").as_bytes();
  ```
- 使用 `#[used]` 防止链接器优化掉（即使未被显式引用）
- 若 rusl 不需要暴露此符号给 C 调用者，也可使用 `pub(crate)` 可见性并省略 `#[no_mangle]`
- 注意：C 侧的类型为 `const char __libc_version[]`（字符数组），在 Rust 侧对应 `&[u8]`（字节切片引用）。两者在只读数据段的布局是兼容的：都以 NUL 结尾