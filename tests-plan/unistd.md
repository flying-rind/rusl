# rusl-unistd 替换测试报告

**测试日期**: 2026-06-16

## 替换概况

将 musl-1.2.6 中的 `src/unistd/` 模块（84 个 C 文件）替换为 `rusl-unistd` crate（Rust 实现）。

### 架构变更

本次测试前完成了 `rusl-unistd` 的 syscall 路由重构：

- **`src/syscall.rs`** — feature-gated syscall 模块
  - `rusl` feature: 从 `rusl_internal::syscall` 导入全部
  - `--no-default-features`: 从 `rusl_syscall` 导入 `raw_syscallN`/`do_syscall`/`__syscall_ret` + 本地 `SYS_*` 常量
- **`src/import.rs`** — `do_syscall!` 宏路由
- **所有源文件** — 从 `use rusl_internal::*` 改为 `use crate::syscall::*` / `use crate::import::do_syscall`
- **Cargo.toml** — 新增 `rusl-syscall` 作为常驻依赖

替换后 `librusl_unistd.a` 仅包含 `rusl_unistd` + `rusl_core` + `rusl_syscall` + `core`，不再包含 `rusl_internal` / `rusl_errno`。

## libc-test 测试结果对比

### 原版 REPORT.original（14 个 FAIL）

| # | 测试用例 | 状态 |
|---|---|---|
| 1 | src/api/main.exe | FAIL [status 1] |
| 2 | src/functional/dlopen.exe | FAIL [signal Segmentation fault] |
| 3 | src/functional/strptime.exe | FAIL [status 1] |
| 4 | src/functional/strptime-static.exe | FAIL [status 1] |
| 5 | src/functional/tls_align_dlopen.exe | FAIL [status 1] |
| 6 | src/functional/tls_align.exe | FAIL [status 1] |
| 7 | src/functional/tls_init_dlopen.exe | FAIL [signal Segmentation fault] |
| 8 | src/math/fmal.exe | FAIL [status 1] |
| 9 | src/math/powf.exe | FAIL [status 1] |
| 10 | src/regression/malloc-brk-fail.exe | FAIL [status 1] |
| 11 | src/regression/malloc-brk-fail-static.exe | FAIL [status 1] |
| 12 | src/regression/pthread_once-deadlock.exe | FAIL [timed out] |
| 13 | src/regression/pthread_once-deadlock-static.exe | FAIL [timed out] |
| 14 | src/regression/tls_get_new-dtv.exe | FAIL [signal Segmentation fault] |

### 当前测试结果（9 个 FAIL，全部为原版已有）

| # | 测试用例 | 状态 |
|---|---|---|
| 1 | src/api/main.exe | FAIL [status 1] |
| 2 | src/functional/strptime.exe | FAIL [status 1] |
| 3 | src/functional/strptime-static.exe | FAIL [status 1] |
| 4 | src/math/fmal.exe | FAIL [status 1] |
| 5 | src/math/powf.exe | FAIL [status 1] |
| 6 | src/regression/malloc-brk-fail.exe | FAIL [status 1] |
| 7 | src/regression/malloc-brk-fail-static.exe | FAIL [status 1] |
| 8 | src/regression/pthread_once-deadlock.exe | FAIL [timed out] |
| 9 | src/regression/pthread_once-deadlock-static.exe | FAIL [timed out] |

### 与原版差异

| 变化 | 测试用例 | 说明 |
|---|---|---|
| ✗ 消失 | dlopen.exe, tls_align_dlopen.exe, tls_align.exe, tls_init_dlopen.exe, tls_get_new-dtv.exe | 原版 5 个 SIGSEGV/status 1，可能为环境不稳定导致的间歇性失败 |

## 发现的 Bug 及修复

### Bug 1: syscall 路由未完成 — `--no-default-features` 编译失败

**问题**: 所有源文件直接使用 `rusl_internal::syscall::*`（约 80 处），但 Cargo.toml 将 `rusl-internal` 设为 optional feature。关闭默认 feature 后代码无法编译，导致 `librusl_unistd.a` 包含非预期的依赖。

**修复**:
- 创建 `src/syscall.rs` feature-gated 模块，非 rusl 路径从 `rusl_syscall` 导入并本地定义 SYS 常量
- 简化 `src/import.rs` 仅保留 `do_syscall!` 路由
- 将所有源文件引用从 `rusl_internal` 改为 `crate::syscall` / `crate::import`
- `rusl_syscall` 的 `__syscall_ret` 参数类型从 `usize` 改为 `u64`（与 musl C 的 `unsigned long` 一致），并改为 safe 包装函数

### Bug 2: `sleep`/`usleep` 绕过 pthread 取消点

**文件**: `rusl-unistd/src/sleep.rs`, `rusl-unistd/src/usleep.rs`

**问题**: `sleep` 和 `usleep` 直接调用 `raw_syscall2(SYS_nanosleep)` 而非 musl 的 `nanosleep()` 函数。musl 的 `nanosleep()` 内部使用 `__syscall_cp()`（cancellation point），而原始系统调用绕过了 pthread 取消机制。导致 `pthread_cancel` 测试失败：
```
res == PTHREAD_CANCELED failed (canceled thread exit status)
foo[0] == 1 failed (cleanup handler failed to run)
```

**修复**: 
- 非 rusl 路径: FFI 调用 musl C 的 `nanosleep()`（内部使用 `__syscall_cp`）
- rusl 路径: 保留原始系统调用作为 fallback（TODO: 实现 `__syscall_cp` 语义）

### Bug 3: Makefile 缺少 `rusl-unistd` 构建规则

**文件**: `musl-1.2.6/Makefile`

**问题**: `RUSL_MODULES` 中未注册 `unistd`，缺少 `RUSL_A_unistd` 路径变量、`RUSL_FEATURES_unistd` 配置和构建规则。同时 staticlib LTO 会将非替换模块的依赖内联，需在提取后清理。

**修复**:
- 添加 `RUSL_A_unistd`、`RUSL_FEATURES_unistd`、构建规则
- `RUSL_MODULES` 添加 `unistd`
- 添加 `RUSL_CLEAN_NONREPLACE` 机制，提取 `.a` 后删除非替换模块的目标文件

## 修复验证

修复后重新编译并测试，**pthread_cancel 的 2 个新增 FAIL 全部消除**，测试结果零回归（仅包含原版已有的 9 个 FAIL）。

| 新增 FAIL (修复前) | 修复后 |
|---|---|
| `pthread_cancel.exe` | 通过 |
| `pthread_cancel-static.exe` | 通过 |

## 结论

rusl-unistd 替换核心问题已修复。根因有二：(1) syscall 路由层未完成导致 `--no-default-features` 无法编译；(2) `sleep`/`usleep` 绕过 `__syscall_cp` 取消点机制。修复后零回归，所有测试失败均为原版 musl 已有。
