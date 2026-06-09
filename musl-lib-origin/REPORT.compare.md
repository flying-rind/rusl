# libc-test 对比报告

## 测试环境
- 架构: x86_64
- 测试套件: libc-test (476 个测试二进制)
- 测试日期: 2026-06-09

## 对比结果

| | 原版 musl libc | musl + rusl-malloc |
|---|---|---|
| 失败数 | **14** | **18** |
| 通过率 | **97.1%** | **96.2%** |
| 编译错误 | 0 | 0 |

## 原版 musl libc 自身失败 (14 个)

这些是 musl libc 本身已知问题，与 rusl-malloc 无关：

| 测试 | 原因 |
|------|------|
| `api/main.exe` | status 1 |
| `functional/dlopen.exe` | Segmentation fault |
| `functional/strptime.exe` / `strptime-static.exe` | status 1 |
| `functional/tls_align.exe` | status 1 |
| `functional/tls_align_dlopen.exe` | status 1 |
| `functional/tls_init_dlopen.exe` | Segmentation fault |
| `math/fmal.exe` | status 1 |
| `math/powf.exe` | status 1 |
| `regression/malloc-brk-fail.exe` / `-static.exe` | status 1 |
| `regression/pthread_once-deadlock.exe` / `-static.exe` | timed out |
| `regression/tls_get_new-dtv.exe` | Segmentation fault |

## rusl-malloc 引入的额外失败 (4 个)

| 测试 | 原因 |
|------|------|
| `functional/popen.exe` / `popen-static.exe` | Segmentation fault |
| `functional/spawn.exe` / `spawn-static.exe` | Segmentation fault |
