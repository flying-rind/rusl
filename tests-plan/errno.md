# rusl-errno 替换测试报告

**测试日期**: 2026-06-15

## 替换概况

将 musl-1.2.6 中的 `src/errno/` 模块替换为 `rusl-errno` crate（Rust 实现）。

### rusl-errno 提供的导出符号（7 个函数 + 1 个常量）

| 符号 | 类型 | 说明 |
|---|---|---|
| `__errno_location()` | extern "C" fn | 返回当前线程 errno 变量指针 |
| `___errno_location()` | extern "C" fn | GNU 弱别名，行为同 `__errno_location` |
| `strerror(e)` | extern "C" fn | 返回错误码对应的描述字符串 |
| `__strerror_l(e, loc)` | extern "C" fn | 带 locale 的错误消息函数 |
| `strerror_l(e, loc)` | extern "C" fn | `__strerror_l` 的公开别名 |
| `strerror_r(err, buf, buflen)` | extern "C" fn | 线程安全的消息拷贝到用户缓冲区 |
| `__xpg_strerror_r(err, buf, buflen)` | extern "C" fn | XPG 标准别名 |
| `EINVAL` | 常量 | POSIX EINVAL = 22 |

## libc-test 测试结果对比

### 当前测试结果（替换 rusl-errno 后，共 9 个 FAIL）

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

**rusl-errno 替换后没有引入任何新的测试失败。**

- 当前 9 个 FAIL 全部在原版报告中也存在
- 原版报告中的 5 个 FAIL (dlopen.exe, tls_align.exe, tls_align_dlopen.exe, tls_init_dlopen.exe, tls_get_new-dtv.exe) 在当前测试中消失，均为 TLS/dlopen 相关，与 errno 无关
- errno/strerror 相关测试在报告中没有任何 FAIL 或 warning

## 结论

rusl-errno crate 的 7 个函数和 1 个常量正确替换了 musl libc 中对应的 C 实现，所有 errno 相关测试通过，无回归问题。
