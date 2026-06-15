# rusl-env 替换测试报告

**测试日期**: 2026-06-15（修复后更新）

## 替换概况

将 musl-1.2.6 中的 `src/env/` 模块替换为 `rusl-env` crate（Rust 实现）。

### rusl-env 提供的 C ABI 导出符号（22 个）

| 符号 | 类型 | 说明 |
|---|---|---|
| `getenv` | fn | 获取环境变量 |
| `putenv` | fn | 设置环境变量（旧版） |
| `setenv` | fn | 设置环境变量（POSIX） |
| `unsetenv` | fn | 删除环境变量 |
| `clearenv` | fn | 清空所有环境变量 (GNU) |
| `secure_getenv` | fn | 安全获取环境变量 (GNU) |
| `environ` / `__environ` | static / asm | 环境变量指针数组 |
| `_start` | asm | ELF 入口点 (x86_64) |
| `_start_c` | fn | CRT 启动 C 层入口 |
| `__libc_start_main` | fn | libc 主启动函数 |
| `__init_libc` | fn | 初始化 libc 全局状态 |
| `_init` | fn (weak) | System V ABI 初始化 |
| `__stack_chk_fail` | fn | 栈保护器失败处理 |
| `__stack_chk_fail_local` | fn | 本地栈保护器失败 |
| `__stack_chk_guard` | static mut | 栈金丝雀值 |
| `__init_ssp` | fn | 初始化栈保护器 |
| `__init_tp` | fn | 初始化线程指针 |
| `__copy_tls` | fn | 复制 TLS 数据 |
| `__reset_tls` | fn | 重置 TLS（fork 后） |
| `__thread_list_lock` | static mut | 线程列表锁 |

## libc-test 测试结果对比（修复后）

### 当前测试结果（共 10 个 FAIL，全部为原版已有）

| # | 测试用例 | 状态 |
|---|---|---|
| 1 | src/api/main.exe | FAIL [status 1] |
| 2 | src/functional/strptime-static.exe | FAIL [status 1] |
| 3 | src/functional/strptime.exe | FAIL [status 1] |
| 4 | src/functional/tls_align.exe | FAIL [status 1] |
| 5 | src/math/fmal.exe | FAIL [status 1] |
| 6 | src/math/powf.exe | FAIL [status 1] |
| 7 | src/regression/malloc-brk-fail-static.exe | FAIL [status 1] |
| 8 | src/regression/malloc-brk-fail.exe | FAIL [status 1] |
| 9 | src/regression/pthread_once-deadlock-static.exe | FAIL [timed out] |
| 10 | src/regression/pthread_once-deadlock.exe | FAIL [timed out] |

## 发现的 Bug 及修复

### Bug 1: `Pthread.locale` 字段类型错误

**文件**: `rusl-internal/src/pthread_impl.rs`

**问题**: `locale` 字段定义为 `Locale([u8; 64])`（64 字节内联结构体），但 musl 中 `locale_t` = `struct __locale_struct *`（8 字节指针）。这导致 Pthread 结构体布局从 locale 字段开始偏移 56 字节，`killlock`、`dlerror_buf`、`stdio_locks` 等后续字段全部偏移。

**修复**: 将 `pub locale: Locale` 改为 `pub locale: *mut c_void`（8 字节指针），与 musl ABI 一致。

### Bug 2: `init_tp` 三个初始化赋值错误

**文件**: `rusl-env/src/__init_tls.rs`

| 字段 | 错误值 | 正确值 (匹配 musl) |
|---|---|---|
| `locale` | `core::mem::zeroed()` (NULL) | `&libc.global_locale` |
| `sysinfo` | `0` | `__sysinfo` (vDSO 地址) |
| `set_tid_address` 目标 | `&THREAD_LIST_LOCK` (SpinLock) | `&__thread_list_lock` (volatile int) |

**影响**:
- `locale = NULL` → `uselocale()` 失败（uselocale-0 测试）
- `sysinfo = 0` → vDSO 调用回退到慢速系统调用路径
- `set_tid_address` 指向 SpinLock → 内核在线程退出时清零 SpinLock 内部状态，可能导致锁竞争异常

### Bug 3: `import.rs` 非 rusl 回退的两个问题

**文件**: `rusl-env/src/import.rs`

- `DEFAULT_STACK_SIZE`: 80KB → 128KB（匹配 musl `DEFAULT_STACK_SIZE = 131072`）
- `__pthread_self()` x86_64: 移除错误的 `sizeof(Pthread)` 减法（x86_64 上 `fs:0` 已是 Pthread 指针）

## 修复验证

修复后重新编译并测试，**7 个新增 FAIL 全部消除**，测试结果与原版报告一致（仅包含原版已有的 10 个 FAIL）。

| 新增 FAIL (修复前) | 修复后 |
|---|---|
| `random-static.exe` | 通过 |
| `strtold-static.exe` | 通过 |
| `udiv-static.exe` | 通过 |
| `malloc-oom-static.exe` | 通过 |
| `setenv-oom-static.exe` | 通过 |
| `uselocale-0-static.exe` | 通过 |
| `uselocale-0.exe` | 通过 |

## 结论

rusl-env 替换核心问题已修复。根因是 `Pthread` 结构体 `locale` 字段的类型与 musl C ABI 不匹配（内联结构体 vs 指针），以及 `init_tp` 中的初始化赋值不完整。修复后零回归，所有测试失败均为原版 musl 已有。
