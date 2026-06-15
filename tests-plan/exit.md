# rusl-exit 替换测试报告

**测试日期**: 2026-06-15

## 替换概况

将 musl-1.2.6 中的 `src/exit/` 模块替换为 `rusl-exit` crate（Rust 实现）。

### rusl-exit 提供的 C ABI 导出符号（约 12 个）

| 符号 | 说明 |
|---|---|
| `exit(code)` | 标准进程终止，含 atexit 处理、.fini_array 析构、stdio 刷新 |
| `_Exit(ec)` | 立即终止进程，无任何清理 |
| `abort()` | 异常终止（SIGABRT） |
| `__assert_fail(expr, file, line, func)` | 断言失败处理 |
| `atexit(func)` | 注册退出回调函数 |
| `__cxa_atexit(func, arg, dso)` | C++ atexit 注册 |
| `__cxa_finalize(dso)` | 执行特定 DSO 的回调 |
| `__funcs_on_exit()` | 执行所有注册的退出回调 |
| `quick_exit(code)` | 快速退出（ISO C11） |
| `at_quick_exit(func)` | 注册快速退出回调 |
| `__funcs_on_quick_exit()` | 执行快速退出回调 |
| `__atexit_lockptr` / `__at_quick_exit_lockptr` | 退出回调锁指针 |
| `__abort_lock` | abort 互斥锁 |

## libc-test 测试结果对比

### 当前测试结果（替换 rusl-exit 后，共 9 个 FAIL）

| # | 测试用例 | 状态 |
|---|---|---|
| 1 | src/api/main.exe | FAIL [status 1] |
| 2 | src/functional/strptime-static.exe | FAIL [status 1] |
| 3 | src/functional/strptime.exe | FAIL [status 1] |
| 4 | src/math/fmal.exe | FAIL [status 1] |
| 5 | src/math/powf.exe | FAIL [status 1] |
| 6 | src/regression/malloc-brk-fail-static.exe | FAIL [status 1] |
| 7 | src/regression/malloc-brk-fail.exe | FAIL [status 1] |
| 8 | src/regression/pthread_once-deadlock-static.exe | FAIL [timed out] |
| 9 | src/regression/pthread_once-deadlock.exe | FAIL [timed out] |

### 原版测试结果（14 个 FAIL）

| # | 测试用例 | 状态 |
|---|---|---|
| 1 | src/api/main.exe | FAIL [status 1] |
| 2 | src/functional/dlopen.exe | FAIL [signal Segmentation fault] |
| 3 | src/functional/strptime-static.exe | FAIL [status 1] |
| 4 | src/functional/strptime.exe | FAIL [status 1] |
| 5 | src/functional/tls_align.exe | FAIL [status 1] |
| 6 | src/functional/tls_align_dlopen.exe | FAIL [status 1] |
| 7 | src/functional/tls_init_dlopen.exe | FAIL [signal Segmentation fault] |
| 8 | src/math/fmal.exe | FAIL [status 1] |
| 9 | src/math/powf.exe | FAIL [status 1] |
| 10 | src/regression/malloc-brk-fail-static.exe | FAIL [status 1] |
| 11 | src/regression/malloc-brk-fail.exe | FAIL [status 1] |
| 12 | src/regression/pthread_once-deadlock-static.exe | FAIL [timed out] |
| 13 | src/regression/pthread_once-deadlock.exe | FAIL [timed out] |
| 14 | src/regression/tls_get_new-dtv.exe | FAIL [signal Segmentation fault] |

## 新增错误分析

**rusl-exit 替换后没有引入任何新的测试失败。**

- 当前 9 个 FAIL 全部在原版报告中也存在
- 原版报告中的 5 个 FAIL (dlopen.exe, tls_align.exe, tls_align_dlopen.exe, tls_init_dlopen.exe, tls_get_new-dtv.exe) 在当前测试中消失，均为 TLS/dlopen 相关，与 exit 无关
- exit 相关的进程终止、atexit 回调等功能均正常工作

## 结论

rusl-exit crate 正确替换了 musl libc 中对应的 C 实现，所有 exit 相关测试通过，无回归问题。
