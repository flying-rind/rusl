# rusl-internal 替换测试报告

**测试日期**: 2026-06-15

## 替换概况

将 musl-1.2.6 中的 `src/internal/` 模块替换为 `rusl-internal` crate（Rust 实现）。

### rusl-internal 提供的关键 C ABI 导出符号（16 个）

| 符号 | 类型 | 说明 |
|---|---|---|
| `__sysinfo` | static mut | vDSO 页面地址 |
| `__libc` | static mut | 全局 libc 状态 |
| `__hwcap` | static mut | 硬件能力标志 |
| `__progname` | static mut | 短程序名 |
| `__progname_full` | static mut | 完整程序名 |
| `__libc_version` | static | libc 版本字符串 "1.2.6" |
| `__vdsosym` | fn | vDSO 符号查找 |
| `__syscall_ret` | fn | 系统调用返回值转换 |
| `__aio_close` | fn | AIO fd 关闭 |
| `__aio_atfork` | fn | AIO fork 处理 |
| `__floatscan_impl` | fn | 浮点字符串扫描 |
| `__intscan` | fn | 整数字符串扫描 |
| `__procfdname` | fn | /proc/self/fd/N 路径构造 |
| `__pthread_self` | fn | 当前线程 TCB 指针 |
| `__shlim` | fn | 扫描缓冲区限制设置 |
| `__shgetc` | fn | 扫描字符输入 |

## libc-test 测试结果对比

### 当前测试结果（替换 rusl-internal 后，共 25 个 FAIL）

| # | 测试用例 | 状态 | 变化 |
|---|---|---|---|
| 1 | src/api/main.exe | FAIL [status 1] | 原版存在 |
| 2 | src/functional/strptime-static.exe | FAIL [status 1] | 原版存在 |
| 3 | src/functional/strptime.exe | FAIL [status 1] | 原版存在 |
| 4 | src/math/fmal.exe | FAIL [status 1] | 原版存在 |
| 5 | src/math/powf.exe | FAIL [status 1] | 原版存在 |
| 6 | src/regression/malloc-brk-fail-static.exe | FAIL [status 1] | 原版存在 |
| 7 | src/regression/malloc-brk-fail.exe | FAIL [status 1] | 原版存在 |
| 8 | src/regression/pthread_once-deadlock-static.exe | FAIL [timed out] | 原版存在 |
| 9 | src/regression/pthread_once-deadlock.exe | FAIL [timed out] | 原版存在 |
| **10** | **src/functional/fscanf-static.exe** | **FAIL [status 1]** | **新增** |
| **11** | **src/functional/fscanf.exe** | **FAIL [status 1]** | **新增** |
| **12** | **src/functional/fwscanf-static.exe** | **FAIL [status 1]** | **新增** |
| **13** | **src/functional/fwscanf.exe** | **FAIL [status 1]** | **新增** |
| **14** | **src/functional/sscanf-static.exe** | **FAIL [status 1]** | **新增** |
| **15** | **src/functional/sscanf.exe** | **FAIL [status 1]** | **新增** |
| **16** | **src/functional/strtod-static.exe** | **FAIL [status 1]** | **新增** |
| **17** | **src/functional/strtod.exe** | **FAIL [status 1]** | **新增** |
| **18** | **src/functional/strtod_long-static.exe** | **FAIL [status 1]** | **新增** |
| **19** | **src/functional/strtod_long.exe** | **FAIL [status 1]** | **新增** |
| **20** | **src/functional/strtod_simple-static.exe** | **FAIL [status 1]** | **新增** |
| **21** | **src/functional/strtod_simple.exe** | **FAIL [status 1]** | **新增** |
| **22** | **src/functional/strtof-static.exe** | **FAIL [status 1]** | **新增** |
| **23** | **src/functional/strtof.exe** | **FAIL [status 1]** | **新增** |
| **24** | **src/functional/strtold-static.exe** | **FAIL [status 1]** | **新增** |
| **25** | **src/functional/strtold.exe** | **FAIL [status 1]** | **新增** |

## 新增错误分析（16 个新增 FAIL）

所有新增错误均与 `rusl-internal` 中的扫描函数实现有关，可分为两类：

### 1. 整数扫描问题 (`__intscan`) — 6 个测试

- `fscanf`, `fwscanf`, `sscanf` (dynamic + static)
- **症状**: `%i` 解析器识别了过多字段；`%x` 和 `%n` 返回错误结果
- **根因**: `intscan.rs` 中 `__intscan` 的实现与 musl C 版本行为不完全一致

### 2. 浮点扫描问题 (`__floatscan_impl`) — 10 个测试

- `strtod`, `strtod_long`, `strtod_simple`, `strtof`, `strtold` (dynamic + static)
- **症状**:
  - **次正规数处理错误**: 极小的次正规数值返回 `0x0p+0` 而非正确的非零值
  - **大数溢出错误**: 超过 `f64::MAX` 的值返回错误结果而非 `inf`
  - **精度偏差**: 多位有效数字的浮点串转换产生 ulp 级别误差

具体错误示例:

```
strtod(".2470328229206232720882843964341106861825299013071623822127928412503377536351044e-323")
  want 0x1p-1074  got 0x0p+0

strtod("1797693134862315807937289714053034150799341327100378269361737789804449682927647509...")
  want inf  got 0x1p+64
```

## 结论

**rusl-internal 替换引入了 16 个新增测试失败，全部与扫描函数有关。** 问题集中在 `__floatscan_impl` (浮点解析) 和 `__intscan` (整数解析) 的 Rust 实现与 musl C 版本存在行为差异：

- **次正规浮点数和溢出处理**不正确（`floatscan.rs`）
- **整数扫描格式解析**行为有差异（`intscan.rs`）

其他 internal 符号（`__libc`、`__progname`、`__syscall_ret`、`__pthread_self` 等）未引起测试失败，这些符号的实现可能是正确的。
