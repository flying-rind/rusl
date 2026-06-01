# rusl 多 Crate 拆分方案

## 目标

将当前单一体 `rusl` crate 拆分为多个独立 crate，按功能域组织（`rusl-string`、`rusl-regex` 等），并将共同依赖的公有基础设施抽象为 `rusl-core` crate。所有 crate 均沿用当前的自定义 `test!` 测试框架。

---

## 一、现状分析

### 1.1 当前 rusl 模块结构

```
rusl/
  Cargo.toml          # 单一 crate: no_std, staticlib + lib, custom_test_frameworks
  build.rs            # c-test 模式链接 musl libc; 非 c-test 编译 C 胶水代码
  .cargo/config.toml  # build-std = ["core", "alloc"], panic=abort
  Makefile            # test / test-c / inte-test / lib-test 目标
  src/
    lib.rs            # crate root: panic_handler, 条件编译, re-exports
    framework.rs      # 测试框架: test! 宏, setjmp/longjmp, print!/println!, runner
    c_types.rs        # C 类型别名 (size_t, wchar_t, ...) — 纯定义，零依赖
    prelude.rs        # alloc 重导出 (Box, String, Vec)
    arch/             # 架构相关 (x86_64 syscall 原语)
    syscall/          # 系统调用封装 (依赖 arch)
    errno/            # errno 处理 (叶节点)
    exit/             # 进程终止 (叶节点)
    string/           # 字符串/内存操作 — 73 个函数 (依赖 c_types, test!)
    stdlib/           # 标准库工具 (依赖 c_types, internal, string)
    ctype/            # 字符分类 (依赖 c_types, internal::libc::__locale_struct)
    malloc/           # 内存分配器 — mallocng + lite_malloc (依赖 arch, syscall, errno, c_types)
    regex/            # 正则/glob/fnmatch (依赖 c_types, errno, stdlib, string, malloc)
    prng/             # 伪随机数 (依赖 c_types)
    search/           # 搜索/哈希/二叉树 (依赖 stdlib, string)
    internal/         # 内部基础设施 — libc, atomic, futex, pthread_impl 等
    env/              # 环境变量 + CRT 启动 (依赖 c_types, syscall, internal, string)
    stdio/            # 标准 I/O (依赖 c_types, errno, stdlib, string, internal)
    unistd/           # POSIX 系统调用封装 (依赖 arch, syscall)
    api/              # c-test 模式下的 extern "C" 声明 (仅在 c-test feature 时编译)
  tests/
    integration.rs    # 单一集成测试入口 (#![no_std], custom_test_frameworks)
    common/mod.rs     # 双模式符号导入 (Rust impl vs musl libc extern "C")
    string/           # 每个域的测试文件
    ctype/
    stdlib/
    malloc/
    prng/
    search/
    internal/
    env/
    errno/
```

### 1.2 模块间依赖关系 (当前)

```
c_types       — 零依赖
framework     — 零外部依赖 (仅 core::*, 内联 asm)
arch          — 零依赖
syscall       — → arch
errno         — 零依赖
exit          — 零依赖
prelude       — → alloc

internal      — → c_types (子模块有复杂内部依赖)
string        — → c_types (类型), test! (宏)
stdlib        — → c_types, internal, string
ctype         — → c_types, internal::libc::__locale_struct
malloc        — → arch, syscall, errno, c_types
prng          — → c_types
search        — → stdlib, string
regex         — → c_types, errno, stdlib, string, malloc
env           — → c_types, syscall, internal, string
stdio         — → c_types, errno, stdlib, string, internal
unistd        — → arch, syscall
```

### 1.3 测试框架核心要素 (framework.rs)

当前测试框架是 **自包含的零依赖模块**，它提供：

| 组件 | 说明 |
|------|------|
| `JmpBuf` | setjmp/longjmp 寄存器保存结构 (x86_64) |
| `__rusl_setjmp` / `__rusl_longjmp` | global_asm! 实现的上下文保存/恢复 |
| `sys_write` / `sys_exit` | Linux x86_64 系统调用 |
| `Stdout` | 实现 `core::fmt::Write`，通过 sys_write 输出 |
| `print!` / `println!` | 宏，使用 `$crate::framework::Stdout` |
| `test!` | 宏，展开为 `#[test_case]` + `framework::run_test` |
| `runner` | 测试运行器，setjmp 点捕获 panic，longjmp 恢复 |
| `test_panic_handler` | panic 时打印失败信息并 longjmp |
| `install_panic_hook` | 注入 panic hook 到 lib crate 的 `#[panic_handler]` |

## 二、拆分方案

### 2.1 总体架构

采用 **Cargo workspace** 组织，所有 crate 的 `#![no_std]`、自定义测试框架、`panic=abort` 等配置保持一致。

```
rusl/                          # workspace 根目录
  Cargo.toml                   # [workspace] 定义, members = [...]
  .cargo/config.toml           # 共享 build 配置
  Makefile                     # 工作空间级构建/测试命令
  
  rusl-core/                   # === 共享基础设施 ===
    Cargo.toml
    src/
      lib.rs                   # crate root: panic_handler, re-exports
      c_types.rs               # C 类型别名 (从当前 src/c_types.rs 平移)
      framework.rs             # 测试框架 (从当前 src/framework.rs 平移)
      prelude.rs               # alloc 重导出 (从当前 src/prelude.rs 平移)
      arch/                    # 架构相关 syscall 原语
      syscall/                 # 系统调用封装
      errno/                   # errno 处理

  rusl-internal/               # === 内部基础设施 ===
    Cargo.toml
    src/
      lib.rs
      libc.rs                  # libc 全局状态
      atomic.rs                # 原子操作
      futex.rs                 # futex 常量
      version.rs               # 版本号
      defsysinfo.rs
      lock.rs
      complex_impl.rs
      procfdname.rs
      aio_impl.rs
      fork_impl.rs
      emulate_wait4.rs
      floatscan.rs
      intscan.rs
      ksigaction.rs
      shcall.rs
      shgetc.rs
      fdpic_crt.rs
      pthread_impl.rs          # pthread 内部结构

  rusl-string/                 # === 字符串/内存操作 ===
    Cargo.toml
    src/                       # 从当前 src/string/ 平移
    tests/                     # 从当前 tests/string/ 平移

  rusl-stdlib/                 # === 标准库工具 ===
    Cargo.toml
    src/                       # 从当前 src/stdlib/ 平移
    tests/                     # 从当前 tests/stdlib/ 平移

  rusl-ctype/                  # === 字符分类 ===
    Cargo.toml
    src/                       # 从当前 src/ctype/ 平移
    tests/                     # 从当前 tests/ctype/ 平移

  rusl-malloc/                 # === 内存分配器 ===
    Cargo.toml
    src/                       # 从当前 src/malloc/ 平移
    tests/                     # 从当前 tests/malloc/ 平移

  rusl-regex/                  # === 正则表达式/通配符匹配 ===
    Cargo.toml
    src/                       # 从当前 src/regex/ 平移
    tests/                     # (当前无独立 regex 集成测试，需新建)

  rusl-prng/                   # === 伪随机数 ===
    Cargo.toml
    src/                       # 从当前 src/prng/ 平移
    tests/                     # 从当前 tests/prng/ 平移

  rusl-search/                 # === 搜索/哈希表/二叉树 ===
    Cargo.toml
    src/                       # 从当前 src/search/ 平移
    tests/                     # 从当前 tests/search/ 平移

  rusl-env/                    # === 环境变量 + CRT 启动 ===
    Cargo.toml
    src/                       # 从当前 src/env/ 平移
    tests/                     # 从当前 tests/env/ 平移

  rusl-unistd/                 # === POSIX 系统调用封装 ===
    Cargo.toml
    src/                       # 从当前 src/unistd/ 平移

  rusl-stdio/                  # === 标准 I/O ===
    Cargo.toml
    src/                       # 从当前 src/stdio/ 平移

  rusl-exit/                   # === 进程终止 ===
    Cargo.toml
    src/                       # 从当前 src/exit/ 平移

  rusl/                        # === 顶层聚合 crate (保留) ===
    Cargo.toml                 # 依赖所有域 crate, staticlib 产出
    build.rs                   # c-test 链接 musl libc.a
    src/
      lib.rs                   # 聚合 re-export, 统一 panic_handler
      api/                     # c-test extern "C" 声明 (可细分为子模块)
    tests/
      integration.rs           # 跨域集成测试入口
      common/mod.rs            # 双模式符号导入
```

### 2.2 各 crate 详细定义

#### 2.2.1 rusl-core (共享基础设施)

```
rusl-core/
  Cargo.toml
    [package]
    name = "rusl-core"
    version = "0.1.0"
    edition = "2021"

    [lib]
    crate-type = ["lib"]        # 仅 lib, 不产出 staticlib
    doctest = false
    test = true

    [features]
    default = []
    c-test = []                 # 启用时抑制内部实现, 暴露 extern "C" 声明

    [dependencies]
    # 纯 no_std, 零外部依赖
    # 使用 core + alloc

  src/
    lib.rs
      #![no_std]
      #![feature(custom_test_frameworks)]
      #![test_runner(rusl_core::framework::runner)]
      #![reexport_test_harness_main = "test_main"]
      #![no_main]

      extern crate alloc;

      // panic_handler — 同当前 lib.rs
      // PANIC_HOOK — 同当前 lib.rs
      // __rusl_set_panic_hook — 同当前 lib.rs

      pub mod c_types;
      pub mod framework;
      pub mod prelude;
      pub mod arch;
      pub mod syscall;
      pub mod errno;

      #[cfg(test)]
      #[no_mangle]
      pub extern "C" fn main(...) -> i32 { test_main(); 0 }

    c_types.rs        — 平移自当前 src/c_types.rs
    framework.rs      — 平移自当前 src/framework.rs
    prelude.rs        — 平移自当前 src/prelude.rs
    arch/mod.rs       — 平移自当前 src/arch/mod.rs
    arch/x86_64.rs    — 平移
    syscall/mod.rs    — 平移自当前 src/syscall/, 调整 use crate::arch → use crate::arch
    errno/mod.rs      — 平移自当前 src/errno/mod.rs
```

**关键变更**: `framework.rs` 中的 `test!` 宏当前展开为 `$crate::framework::run_test(...)`，移动到 `rusl-core` 后，其他 crate 需要通过 `rusl_core::test!` 使用。因此需要在 `rusl-core` 中提供一个跨 crate 使用模式：

```rust
// rusl-core/src/lib.rs 中重新导出宏
pub use framework::{test, print, println};
```

域 crate 使用:
```rust
use rusl_core::{test, print, println};
```

**注意**: `test!` 宏当前为 `#[macro_export]`，导出到 crate root。移动到 `rusl-core` 后，调整为导出自 `rusl_core::test!`。

#### 2.2.2 域 crate 模板 (以 rusl-string 为例)

```
rusl-string/
  Cargo.toml
    [package]
    name = "rusl-string"
    version = "0.1.0"
    edition = "2021"

    [lib]
    crate-type = ["lib", "staticlib"]
    doctest = false
    test = true

    [features]
    default = []
    c-test = []                 # 启用时抑制实现, 暴露 extern "C"

    [dependencies]
    rusl-core = { path = "../rusl-core" }

    [profile.dev]
    panic = "abort"

    [profile.test]
    panic = "abort"

    [profile.release]
    panic = "abort"
    opt-level = "s"
    lto = true

  src/
    lib.rs
      #![no_std]
      #![feature(custom_test_frameworks)]
      #![test_runner(rusl_core::framework::runner)]
      #![reexport_test_harness_main = "test_main"]
      #![no_main]

      extern crate alloc;

      // 不定义独立的 panic_handler — 由 rusl-core 提供
      // (或在链接时由顶层 crate 提供)

      pub use rusl_core::c_types;
      use rusl_core::{test, print, println};

      mod bcmp;
      mod bcopy;
      // ... 所有 string 子模块 (从当前 src/string/ 平移)
      mod wmemset;

      pub use bcmp::bcmp;
      // ... 所有公开重导出

      // 单元测试入口
      #[cfg(test)]
      #[no_mangle]
      pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
          test_main();
          0
      }

    bcmp.rs, bcopy.rs, ...  # 从当前 src/string/*.rs 平移
      // 内部调整: use crate::c_types::* → use rusl_core::c_types::*
      // 内部调整: use crate::test → use rusl_core::test
      // 内部调整: use crate::print* → use rusl_core::print*

  tests/
    mod.rs
    bcmp_test.rs
    bcopy_test.rs
    ...                      # 从当前 tests/string/*.rs 平移
      // 调整: use rusl::common::* → use rusl_string::* (或自行声明符号)
      // 注意: c-test 模式下的 extern "C" 声明需要在该 crate 中重新提供
```

#### 2.2.3 域 crate 依赖关系 (新)

```
rusl-core       — 零外部依赖
rusl-internal   — → rusl-core
rusl-string     — → rusl-core
rusl-ctype      — → rusl-core, rusl-internal
rusl-stdlib     — → rusl-core, rusl-internal, rusl-string
rusl-malloc     — → rusl-core
rusl-prng       — → rusl-core
rusl-search     — → rusl-core, rusl-stdlib
rusl-regex      — → rusl-core, rusl-stdlib, rusl-string, rusl-malloc, rusl-ctype
rusl-env        — → rusl-core, rusl-internal, rusl-string
rusl-unistd     — → rusl-core
rusl-stdio      — → rusl-core, rusl-stdlib, rusl-string, rusl-internal
rusl-exit       — → rusl-core
```

### 2.3 跨 crate 调用约定

来自域 crate A 的 `#[no_mangle] pub extern "C" fn foo(...)` 被域 crate B 依赖时：

- **Rust 层调用**: `rusl_string::foo(...)` — 直接函数调用 (extern "C" fn 在 Rust 中可正常调用)
- **C ABI 链接**: 每个域 crate 产出 `staticlib`，最终由顶层 `rusl` 聚合 crate 统一链接为 `librusl.a`

#### 跨 crate 使用的关键调整点

| 调整项 | 当前写法 | 迁移后写法 |
|--------|---------|-----------|
| 类型引用 | `use crate::c_types::size_t` | `use rusl_core::c_types::size_t` |
| 测试宏 | `use crate::test` | `use rusl_core::test` |
| 输出宏 | `crate::print!(...)` | `rusl_core::print!(...)` |
| 打印宏 | `crate::println!(...)` | `rusl_core::println!(...)` |
| errno | `use crate::errno::...` | `use rusl_core::errno::...` |
| 系统调用 | `use crate::syscall::...` | `use rusl_core::syscall::...` |
| String 函数 | `use crate::string::strlen` | `use rusl_string::strlen` |
| Stdlib 函数 | `use crate::stdlib::abs` | `use rusl_stdlib::abs` |
| Internal 类型 | `use crate::internal::libc::...` | `use rusl_internal::libc::...` |
| malloc 函数 | `use crate::malloc::...` | `use rusl_malloc::...` |
| framework | `use crate::framework::Stdout` | `use rusl_core::framework::Stdout` |

### 2.4 panic_handler 策略

当前 `rusl` 只有一个 `#[panic_handler]` (在 `src/lib.rs` 中)，通过 `PANIC_HOOK` 原子指针支持测试框架注入。

**拆分后**: 每个 crate 都需要自己的 `#[panic_handler]` (因为它们是独立的编译单元)。方案：

- **方案 A (推荐)**: `rusl-core` 提供 panic_handler 实现，域 crate 通过 `extern crate rusl_core;` 使用，并在自己的 `lib.rs` 中不定义 panic_handler。由于 panic_handler 是全局唯一的，链接时由最终二进制选择。

  实际上，`#![no_std]` 的 lib crate 可以不定义 panic_handler，但如果它作为独立的 staticlib 链接，最终二进制必须有 panic_handler。域 crate 作为 lib 类型被顶层 crate 依赖时，不需要自己的 panic_handler。

- **方案 B**: 每个域 crate 定义相同的 panic_handler 模式 (PANIC_HOOK + set_panic_hook)，但这会导致代码重复。

**推荐方案 A**: 只在 `rusl-core` 中定义 panic_handler。域 crate 和测试二进制都依赖 `rusl-core`，链接时使用 `rusl-core` 的 panic_handler。域 crate 的 `lib.rs` 不定义自己的 panic_handler：

```rust
// rusl-string/src/lib.rs
#![no_std]
// 不定义 #[panic_handler] — 由 rusl-core 提供
// 测试时 framework 通过 rusl_core::__rusl_set_panic_hook 注入
```

### 2.5 c-test 功能支持

c-test feature 的核心问题是：需要同时满足 "链接 musl libc" 和 "不编译 Rust 实现以避免符号冲突"。

**设计方案**:

- 每个域 crate 的 `c-test` feature 控制是否编译该域的 Rust 实现:
  ```rust
  // rusl-string/src/lib.rs
  #[cfg(not(feature = "c-test"))]
  mod bcmp;  // ... 实现模块
  
  #[cfg(feature = "c-test")]
  pub mod c_shim;  // extern "C" 声明
  ```

- 顶层 `rusl` crate 的 `c-test` feature 负责:
  1. 启用所有域 crate 的 `c-test` feature (通过 Cargo feature 传递)
  2. `build.rs` 链接 musl libc.a 和启动文件
  3. 提供统一的 extern "C" 声明

- 各域 crate 也可独立运行测试:
  ```bash
  cd rusl-string
  cargo test                          # 测试 Rust 实现
  cargo test --features c-test        # 测试 musl libc C 实现
  ```

**关键问题**: `c-test` 模式下域 crate 的 extern "C" 声明从哪里来？

选项 1: 每个域 crate 维护自己的 `c_shim.rs` (类似当前 `src/api/` 拆分)
选项 2: 集中在一个 `rusl-c-shim` crate 中
选项 3: 顶层 `rusl` crate 统一提供，域 crate 通过 feature gate 做条件编译

**推荐选项 1**: 每个域 crate 维护自己的 c-test shim，因为它是该 crate 自身测试的一部分。同时顶层 `rusl` crate 保留当前 `api/` 模块作为统一的 c-test 入口。

### 2.6 集成测试迁移

当前所有集成测试在 `rusl/tests/` 下的单一测试二进制中。拆分后:

| 测试类型 | 位置 | 说明 |
|---------|------|------|
| 单元测试 | `rusl-<domain>/src/*.rs` (内联 `#[cfg(test)]`) | 使用 `test!` 宏 |
| 域集成测试 | `rusl-<domain>/tests/*.rs` | 测试该域的对外 C ABI 符号 |
| 跨域集成测试 | `rusl/tests/*.rs` (保留) | 测试需要多域协作的场景 |

每个域 crate 的集成测试文件从当前 `tests/<domain>/*.rs` 平移，调整 `common/mod.rs` 引用:

```rust
// 当前: tests/common/mod.rs 通过 rusl:: 引用符号
// 迁移后: 域 crate 的 tests/ 直接使用 rusl_<domain>:: 引用
```

对于 c-test 模式，域 crate 的集成测试需要:
- 非 c-test: 依赖 `rusl-<domain>` 的 Rust 实现
- c-test: 通过 extern "C" 声明使用 musl libc 符号

**c-test 模式下链接 musl libc 的问题**: 域 crate 的 build.rs 需要负责 c-test 链接。每个域 crate 都需要一个类似的 `build.rs` 来处理 c-test 链接。为避免重复，可以将链接逻辑抽象为 `rusl-core` 的一个 build 辅助函数，或者使用 Cargo feature 传递。

**简化方案**: c-test 功能集中在顶层 `rusl` crate 中实现 (保留当前模式)。域 crate 自身的测试只测试 Rust 实现。c-test 集成测试在顶层 `rusl` crate 中进行。这样每个域 crate 不需要知道 musl libc 的存在。

## 三、实施步骤

### 步骤 1: 建立 workspace 结构

1. 创建 workspace 级 `Cargo.toml`:
   ```toml
   [workspace]
   members = [
       "rusl-core",
       "rusl-internal",
       "rusl-string",
       "rusl-stdlib",
       "rusl-ctype",
       "rusl-malloc",
       "rusl-regex",
       "rusl-prng",
       "rusl-search",
       "rusl-env",
       "rusl-unistd",
       "rusl-stdio",
       "rusl-exit",
       "rusl",
   ]
   resolver = "2"
   ```

2. 将当前 `.cargo/config.toml` 提升到 workspace 根目录

3. 创建各 crate 目录骨架 (Cargo.toml + src/lib.rs)

### 步骤 2: 实现 rusl-core

1. 从当前 `src/framework.rs`, `src/c_types.rs`, `src/prelude.rs`, `src/arch/`, `src/syscall/`, `src/errno/` 平移代码
2. 调整 `framework.rs` 中的宏输出 (确保 `test!`/`print!`/`println!` 通过 `rusl_core::` 可用)
3. 实现 panic_handler (保持当前 PANIC_HOOK 模式)
4. 验证: `cd rusl-core && cargo test --lib` 通过

### 步骤 3: 迁移 rusl-string (最简依赖域)

1. 创建 `rusl-string/` 结构
2. `src/` 从 `rusl/src/string/` 平移
3. `tests/` 从 `rusl/tests/string/` 平移
4. 调整所有 `use crate::` 为 `use rusl_core::`
5. 在 `rusl-string` crate 中处理自身的 `#[test_runner]` 和 `test_main` 入口
6. 验证: `cd rusl-string && cargo test` 通过

### 步骤 4: 迁移 rusl-internal

1. 平移 `src/internal/` 下所有模块
2. 处理 `internal` 对 `c_types` 的依赖 (改为 `rusl-core`)

### 步骤 5: 迁移 rusl-stdlib, rusl-ctype (依赖 internal)

1. 平移代码 + 调整引用
2. 处理跨 crate 依赖 (stdlib → string, ctype → internal)

### 步骤 6: 迁移其余域 crate

按依赖顺序: `rusl-prng`, `rusl-malloc`, `rusl-search`, `rusl-env`, `rusl-regex`, `rusl-unistd`, `rusl-stdio`, `rusl-exit`

### 步骤 7: 更新顶层 rusl crate

1. 改为聚合 crate，依赖所有域 crate
2. `build.rs` 保留 c-test 链接逻辑
3. `src/lib.rs` 聚合 re-export 所有符号
4. `src/api/` 保留 c-test 模式 extern "C" 声明
5. `tests/` 保留跨域集成测试

### 步骤 8: 更新 Makefile 和 CI

```makefile
# 构建所有 crate
build-all:
	cargo build --workspace

# 运行所有单元测试
test-all:
	cargo test --workspace --lib

# 运行所有集成测试
inte-test-all:
	cargo test --workspace --test '*'

# 测试 musl libc (仅顶层 crate)
test-c:
	cd rusl && cargo test --features c-test

# 单独测试某个域 crate
test-string:
	cd rusl-string && cargo test

test-string-c:
	cd rusl-string && cargo test --features c-test
```

## 四、待解决问题与风险

### 4.1 panic_handler 双定义风险

**问题**: 如果多个 domain crate 都定义 `#[panic_handler]`，链接时会产生冲突。

**缓解**: 
- 只在 `rusl-core` 中定义 panic_handler
- Domain crate 的 `lib.rs` 使用 `#![no_std]` 但不定义 panic_handler
- 测试二进制通过 `extern crate rusl_core;` 引入 panic_handler
- 顶层 `rusl` crate 可以覆盖 panic_handler (如果需要不同行为)

### 4.2 test! 宏的跨 crate 可用性

**问题**: 当前 `test!` 宏展开依赖 `$crate::framework::run_test()`，在 `rusl-core` 中，`$crate` 是 `rusl_core`。其他 crate 需要使用 `rusl_core::test!`。

**缓解**: 
- `rusl-core` 的 `framework.rs` 中的宏保持 `#[macro_export]`
- 路径使用 `$crate::framework::run_test` — 因为宏定义在 `rusl-core` 中，`$crate` 解析为 `rusl_core`
- Domain crate 通过 `use rusl_core::test;` 导入使用

**验证**: 需要在迁移第一个域 crate 时验证宏展开正确性。

### 4.3 `#[no_mangle]` 符号全局唯一性

**问题**: 如果两个不同的域 crate 定义了相同的 `#[no_mangle]` 符号 (例如两个 crate 中都有 `strlen`)，链接时冲突。

**缓解**: 
- 每个 C ABI 函数只在一个 crate 中定义 (对应 musl 中的源文件归属)
- 其他 crate 通过 crate 依赖引用，而非重复定义
- 代码审查确保无重复 `#[no_mangle]` 定义

### 4.4 循环依赖

**当前风险**: `internal` 的 `libc.rs` 定义 `__locale_struct`，被 `ctype` 和 `stdlib` 使用。`stdlib` 使用 `string` 的函数。不存在循环依赖。

**值得注意的依赖链**: 
- `regex → stdlib → internal → core` — 无循环
- `env → internal → core` — 无循环

当前模块间无循环依赖，拆分后增加 crate 边界不会引入新的循环。

### 4.5 c-test 模式在各域 crate 中的支持

**问题**: 当前 c-test 通过顶层 `rusl` 的 `build.rs` 链接 musl libc，域 crate 自己如何支持 c-test？

**缓解**: 
- 域 crate 自身的 `c-test` feature 用于抑制 Rust 实现编译
- 域 crate 的 `build.rs` 检测 `c-test` feature，链接 musl libc
- 通过 workspace 级 Makefile 统一管理

### 4.6 编译时间

- **优点**: 修改单个域 crate 时只需重新编译该 crate 及其依赖者
- **缺点**: 首次全量编译可能稍慢 (多个 crate 的 metadata 生成)
- **预期**: 总体编译时间减少 (增量编译收益)

## 五、涉及文件清单

### 需要创建的文件 (约 100+)

```
rusl/Cargo.toml                    # workspace 定义 (替换当前)
rusl/.cargo/config.toml            # (从子目录提升)

rusl-core/Cargo.toml
rusl-core/src/lib.rs
rusl-core/src/c_types.rs
rusl-core/src/framework.rs
rusl-core/src/prelude.rs
rusl-core/src/arch/mod.rs
rusl-core/src/arch/x86_64.rs
rusl-core/src/syscall/mod.rs
rusl-core/src/syscall/num.rs
rusl-core/src/syscall/raw.rs
rusl-core/src/syscall/ret.rs
rusl-core/src/errno/mod.rs

rusl-internal/Cargo.toml
rusl-internal/src/lib.rs
rusl-internal/src/libc.rs
rusl-internal/src/atomic.rs
rusl-internal/src/futex.rs
rusl-internal/src/version.rs
rusl-internal/src/defsysinfo.rs
rusl-internal/src/lock.rs
rusl-internal/src/complex_impl.rs
rusl-internal/src/procfdname.rs
rusl-internal/src/aio_impl.rs
rusl-internal/src/fork_impl.rs
rusl-internal/src/emulate_wait4.rs
rusl-internal/src/floatscan.rs
rusl-internal/src/intscan.rs
rusl-internal/src/ksigaction.rs
rusl-internal/src/shcall.rs
rusl-internal/src/shgetc.rs
rusl-internal/src/fdpic_crt.rs
rusl-internal/src/pthread_impl.rs

rusl-string/Cargo.toml
rusl-string/src/lib.rs
rusl-string/src/bcmp.rs
rusl-string/src/bcopy.rs
... (所有 string 子模块, ~73 个文件)
rusl-string/tests/mod.rs
rusl-string/tests/bcmp_test.rs
... (所有 string 集成测试, ~80 个文件)

# 其他域 crate 同理
```

### 需要修改的文件

- 所有域 crate 源文件中的 `use crate::` → `use rusl_core::` / `use rusl_<domain>::`
- 所有 `#[test_runner(crate::framework::runner)]` → `#[test_runner(rusl_core::framework::runner)]`
- 所有集成测试中的 `use rusl::common::*` → 对应域 crate 的导入
- `Makefile` — 添加 workspace 级目标

### 可以删除的文件

- 原 `rusl/src/` 下已迁移到域 crate 的模块 (逐步清理)
- 原 `rusl/tests/<domain>/` 下已迁移的测试 (逐步清理)

## 六、验证标准

1. `cargo build --workspace` 所有 crate 编译成功
2. `cargo test --workspace --lib` 所有单元测试通过
3. `cargo test --workspace --test '*'` 所有集成测试通过
4. `make test-c` (c-test 模式) 顶层集成测试通过
5. 各域 crate 可独立 `cargo test` 通过
6. `rusl/librusl.a` 产出与拆分前功能等价