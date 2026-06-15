# rusl-prng 替换测试报告

**测试日期**: 2026-06-15

## 替换概况

将 musl-1.2.6 中的 `src/prng/` 模块（12 个 C 文件）替换为 `rusl-prng` crate（Rust 实现）。

### rusl-prng 提供的导出符号（16 个函数 + 1 个常量）

| 函数 | 说明 |
|---|---|
| `rand()` | 返回 [0, RAND_MAX] 的伪随机整数 |
| `srand(seed)` | 设置 rand() 全局种子 |
| `rand_r(seed)` | 可重入版 rand() |
| `random()` | 增强随机函数 |
| `srandom(seed)` | 设置 random() 种子 |
| `initstate(seed, state, n)` | 初始化 random() 状态表 |
| `setstate(state)` | 切换 random() 状态表 |
| `drand48()` | 48 位 LCG 浮点随机 |
| `erand48(xsubi)` | 可重入版 drand48() |
| `lrand48()` | 48 位 LCG 非负长整数 |
| `nrand48(xsubi)` | 可重入版 lrand48() |
| `mrand48()` | 48 位 LCG 有符号长整数 |
| `jrand48(xsubi)` | 可重入版 mrand48() |
| `lcong48(p)` | 设置 48 位 LCG 全部参数 |
| `seed48(seed16v)` | 设置 48 位 LCG 种子 |
| `srand48(seedval)` | 通过 32 位种子初始化 48 位 LCG |
| `RAND_MAX` | 常量: 0x7fffffff |

## libc-test 测试结果对比

### 当前测试结果（替换 rusl-prng 后，共 9 个 FAIL）

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

**rusl-prng 替换后没有引入任何新的测试失败。**

- 当前 9 个 FAIL 全部在原版报告中存在
- 原版报告中的 5 个 FAIL（dlopen.exe, tls_align.exe, tls_align_dlopen.exe, tls_init_dlopen.exe, tls_get_new-dtv.exe）在当前测试中消失，均为 TLS/dlopen 相关，与 prng 无关，可能因重新编译环境差异导致
- prng 相关的 `functional/random` 和 `regression/lrand48-signextend` 测试**全部通过**

## 结论

rusl-prng crate 的 16 个函数和 1 个常量正确替换了 musl libc 中对应的 C 实现，所有 prng 相关测试通过，无回归问题。
