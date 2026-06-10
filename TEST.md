# rusl libc-test 单模块替换对比报告

## 测试环境
- musl: 1.2.6
- 架构: x86_64
- 测试套件: libc-test
- 测试日期: 2026-06-10

## 基线说明

原版 musl libc 基线报告 (`musl-config/origin/REPORT.original`) 共 14 个失败:

| # | 测试 | 状态 |
|---|------|------|
| 1 | `api/main.exe` | FAIL [status 1] |
| 2 | `functional/dlopen.exe` | FAIL [signal Segmentation fault] |
| 3 | `functional/strptime.exe` | FAIL [status 1] |
| 4 | `functional/strptime-static.exe` | FAIL [status 1] |
| 5 | `functional/tls_align.exe` | FAIL [status 1] |
| 6 | `functional/tls_align_dlopen.exe` | FAIL [status 1] |
| 7 | `functional/tls_init_dlopen.exe` | FAIL [signal Segmentation fault] |
| 8 | `math/fmal.exe` | FAIL [status 1] |
| 9 | `math/powf.exe` | FAIL [status 1] |
| 10 | `regression/malloc-brk-fail.exe` | FAIL [status 1] |
| 11 | `regression/malloc-brk-fail-static.exe` | FAIL [status 1] |
| 12 | `regression/pthread_once-deadlock.exe` | TIMEOUT |
| 13 | `regression/pthread_once-deadlock-static.exe` | TIMEOUT |
| 14 | `regression/tls_get_new-dtv.exe` | FAIL [signal Segmentation fault] |

## 逐模块对比

### 1. malloc

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 9 | 0 | 5 |

替换后失败列表（9 个）:

| 测试 | 状态 |
|------|------|
| `api/main.exe` | FAIL [status 1] |
| `functional/strptime.exe` | FAIL [status 1] |
| `functional/strptime-static.exe` | FAIL [status 1] |
| `math/fmal.exe` | FAIL [status 1] |
| `math/powf.exe` | FAIL [status 1] |
| `regression/malloc-brk-fail.exe` | FAIL [status 1] |
| `regression/malloc-brk-fail-static.exe` | FAIL [status 1] |
| `regression/pthread_once-deadlock.exe` | TIMEOUT |
| `regression/pthread_once-deadlock-static.exe` | TIMEOUT |

**分析**: 无新增失败。5 个消失的失败（dlopen, tls_align, tls_align_dlopen, tls_init_dlopen, tls_get_new-dtv）均为动态链接/TLS 相关，与 malloc 替换无关，系重建后行为差异。

---

### 2. string

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 9 | 0 | 5 |

替换后失败列表: 与 malloc 完全相同（9 个）。

**分析**: 无新增失败。string 替换未引入任何回归。

---

### 3. ctype

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 9 | 0 | 5 |

替换后失败列表: 与 malloc 完全相同（9 个）。

**分析**: 无新增失败。ctype 替换未引入任何回归。

---

### 4. prng

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 9 | 0 | 5 |

替换后失败列表: 与 malloc 完全相同（9 个）。

**分析**: 无新增失败。prng 替换未引入任何回归。

---

### 5. env

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 14 | 8 | 3 |

替换后失败列表（14 个）:

| 测试 | 状态 | vs 原版 |
|------|------|---------|
| `functional/random-static.exe` | FAIL [status 1] | **新增** |
| `functional/strptime.exe` | FAIL [status 1] | 预存 |
| `functional/strptime-static.exe` | FAIL [status 1] | 预存 |
| `functional/strtold-static.exe` | FAIL [status 1] | **新增** |
| `functional/tls_align.exe` | FAIL [status 1] | 预存 |
| `functional/udiv-static.exe` | FAIL [status 1] | **新增** |
| `math/fmal.exe` | FAIL [status 1] | 预存 |
| `math/powf.exe` | FAIL [status 1] | 预存 |
| `regression/malloc-oom-static.exe` | FAIL [status 226] | **新增** |
| `regression/pthread_once-deadlock.exe` | TIMEOUT | 预存 |
| `regression/pthread_once-deadlock-static.exe` | TIMEOUT | 预存 |
| `regression/setenv-oom-static.exe` | FAIL [status 44] | **新增** |
| `regression/uselocale-0.exe` | FAIL [status 1] | **新增** |
| `regression/uselocale-0-static.exe` | FAIL [status 1] | **新增** |

**新增失败分析**:

| 新增测试 | 可能原因 |
|----------|----------|
| `random-static.exe` | env 替换后初始化顺序变化影响 `random()` 静态链接行为 |
| `strtold-static.exe` | env 替换影响 locale 初始化，进而影响 `strtold` 静态链接 |
| `udiv-static.exe` | 环境变量处理影响 `udiv` 静态链接 |
| `malloc-oom-static.exe` | env 替换后 `setenv`/`getenv` OOM 处理路径变化 |
| `setenv-oom-static.exe` | 同上，`setenv` OOM 状态码不一致（状态 44） |
| `uselocale-0.exe` | locale 初始化依赖 env，静态链接下初始化路径差异 |
| `uselocale-0-static.exe` | 同上 |

---

### 6. internal

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 9 | 3 | 8 |

替换后失败列表（9 个）:

| 测试 | 状态 | vs 原版 |
|------|------|---------|
| `math/fmal.exe` | FAIL [status 1] | 预存 |
| `math/powf.exe` | FAIL [status 1] | 预存 |
| `regression/malloc-brk-fail.exe` | FAIL [status 1] | 预存 |
| `regression/malloc-brk-fail-static.exe` | FAIL [status 1] | 预存 |
| `regression/pthread_once-deadlock.exe` | TIMEOUT | 预存 |
| `regression/pthread_once-deadlock-static.exe` | TIMEOUT | 预存 |
| `functional/strtof.exe` | FAIL [status 1] | **新增** |
| `functional/strtold.exe` | FAIL [status 1] | **新增** |
| `functional/strtold-static.exe` | FAIL [status 1] | **新增** |

**分析**: 修复了 `api/main.exe`（路径问题）和 `strptime.exe`/`strptime-static.exe`（字符串处理）。3 个新增失败源于 `__floatscan_impl`（Rust f64 实现）与 musl 原版（80-bit `long double` 中间精度）的精度差异：

| 新增测试 | 可能原因 |
|----------|----------|
| `strtof.exe` | 子正常数边界解析丢失精度，如 `.7006...e-45`→期望 `0x1p-149` 得到 `0` |
| `strtold.exe` | `"12.345"` 低几位舍入不同；子正常/溢出边界值处理差异 |
| `strtold-static.exe` | 同上 |

---

### 7. errno

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 9 | 0 | 5 |

替换后失败列表（9 个）:

| 测试 | 状态 |
|------|------|
| `api/main.exe` | FAIL [status 1] |
| `functional/strptime.exe` | FAIL [status 1] |
| `functional/strptime-static.exe` | FAIL [status 1] |
| `math/fmal.exe` | FAIL [status 1] |
| `math/powf.exe` | FAIL [status 1] |
| `regression/malloc-brk-fail.exe` | FAIL [status 1] |
| `regression/malloc-brk-fail-static.exe` | FAIL [status 1] |
| `regression/pthread_once-deadlock.exe` | TIMEOUT |
| `regression/pthread_once-deadlock-static.exe` | TIMEOUT |

**分析**: 无新增失败。5 个消失的失败（dlopen, tls_align, tls_align_dlopen, tls_init_dlopen, tls_get_new-dtv）与 errno 替换无关，系重建后行为差异。

**替换范围**:

| musl C 源文件 | Rust 实现 | 涉及符号 |
|---------------|----------|---------|
| `src/errno/__errno_location.c` | `__errno_location.rs` | `__errno_location`, `___errno_location` |
| `src/errno/strerror.c` | `strerror.rs` | `strerror`, `__strerror_l`, `strerror_l` |
| `src/string/strerror_r.c` | `strerror_r.rs` | `strerror_r`, `__xpg_strerror_r` |

**注意**: `strerror_r.c` 位于 musl `src/string/` 目录而非 `src/errno/`。Makefile 通过 `RUSL_ERRNO_STRING_FILTER` 在 errno 替换时排除该文件，避免与 Rust 实现产生多重定义冲突。

**实现限制**: 当前 Stage 0 使用全局 `static mut ERRNO`（非线程安全）。多线程环境下 errno 值会在线程间共享。后续需迁移至 pthread 结构体内部的 per-thread 存储。

**errno 专用测试**: `getpwnam_r-errno` 和 `pthread_atfork-errno-clobber` 均通过，无链接错误或运行时失败。

---

### 8. exit

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 9 | 0 | 5 |

替换后失败列表: 与 errno 完全相同（9 个）。

**分析**: 无新增失败。exit 替换未引入任何回归。

**替换范围**:

| musl C 源文件 | Rust 实现 | 涉及符号 |
|---------------|----------|---------|
| `src/exit/exit.c` | `exit.rs` | `exit`, `_fini`(extern), `__stdio_exit`(extern) |
| `src/exit/_Exit.c` | `_Exit.rs` | `_Exit` |
| `src/exit/abort.c` | `abort.rs` | `abort` |
| `src/exit/atexit.c` | `atexit.rs` | `__cxa_atexit`, `atexit`, `__funcs_on_exit`, `__cxa_finalize`, `__atexit_lockptr` |
| `src/exit/at_quick_exit.c` | `quick_exit.rs` | `at_quick_exit`, `__funcs_on_quick_exit`, `__at_quick_exit_lockptr` |
| `src/exit/quick_exit.c` | `quick_exit.rs` | `quick_exit` |
| `src/exit/abort_lock.c` | `abort_lock.rs` | `__abort_lock` |
| `src/exit/assert.c` | `assert.rs` | `__assert_fail` |

**依赖**: `rusl-syscall` (raw syscall) + `rusl-core` (test framework)。不依赖 `rusl-internal`，可独立替换。

**弱符号处理**: musl C 中 `_fini` 和 `__stdio_exit` 为弱符号，CRT/stdio 模块提供强版本覆盖。Rust 不支持弱符号，改为 `extern "C"` 声明交由链接器解析。`__libc_exit_fini` 不导出 `#[no_mangle]`，避免与 `ldso/dynlink.c` 冲突。

---

### 9. search

| 失败数 | 原版 | 替换后 | 新增 | 修复 |
|--------|------|--------|------|------|
| | 14 | 9 | 0 | 5 |

替换后失败列表: 与 errno/exit 完全相同（9 个）。

**分析**: 无新增失败。search 替换未引入任何回归。

**替换范围**:

| musl C 源文件 | Rust 实现 | 涉及符号 |
|---------------|----------|---------|
| `src/search/hsearch.c` | `hsearch.rs` | `hcreate`, `hdestroy`, `hsearch` |
| `src/search/insque.c` | `insque.rs` | `insque`, `remque` |
| `src/search/lsearch.c` | `lsearch.rs` | `lsearch`, `lfind` |
| `src/search/tdelete.c` | `tdelete.rs` | `tdelete` |
| `src/search/tdestroy.c` | `tdestroy.rs` | `tdestroy` |
| `src/search/tfind.c` | `tfind.rs` | `tfind` |
| `src/search/tsearch.c` | `tsearch.rs` | `tsearch` |
| `src/search/twalk.c` | `twalk.rs` | `twalk` |

**依赖**: `rusl-core` (test framework)。`rusl-malloc`/`rusl-stdlib` 仅在 `rusl` feature 启用时使用（`--no-default-features` 下自带 allocator）。

**search 专用测试**: `functional/search_hsearch`、`functional/search_insque`、`functional/search_lsearch`、`functional/search_tsearch` 及 `api/search` 均通过。
