# emulate_wait4.c 规约

> musl 内部兼容层：在缺少 `SYS_wait4` 系统调用的平台上，用 `SYS_waitid` 模拟 `wait4()`。

---

## 依赖图

```
__emulate_wait4
  ├── __syscall(SYS_waitid, ...)           [来自 syscall.h — 外部模块，跳过]
  ├── __syscall_cp(SYS_waitid, ...)        [来自 syscall.h — 外部模块，跳过]
  ├── <sys/wait.h> 常量 (WEXITED, CLD_*)   [标准头文件 — 外部依赖，跳过]
  ├── <sys/wait.h> 类型 (idtype_t, siginfo_t, P_*) [标准头文件 — 外部依赖，跳过]
  └── (无内部静态函数依赖)
```

---

## 依赖说明

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `__syscall()` | `syscall.h` → 汇编实现 | 外部模块，跳过（系统调用封装） |
| `__syscall_cp()` | `syscall.h` → 汇编实现 | 外部模块，跳过（可取消点系统调用） |
| `SYS_waitid` | `<sys/syscall.h>` | 外部模块（Linux 系统调用号） |
| `WEXITED` | `<sys/wait.h>` | 外部模块（POSIX 常量） |
| `CLD_CONTINUED/D UMPED/EXITED/KILLED/STOPPED/TRAPPED` | `<sys/wait.h>` | 外部模块（POSIX 常量） |
| `P_PGID/P_ALL/P_PID` | `<sys/wait.h>` | 外部模块（POSIX 常量） |
| `idtype_t`, `siginfo_t` | `<sys/wait.h>` | 外部模块（POSIX 类型） |
| `hidden` | musl 编译属性 | 编译器指示 |

---

## __emulate_wait4 (内部函数)

```c
// C 声明 (syscall.h)
hidden long __emulate_wait4(int pid, int *status, int options, void *kru, int cp);
```

```rust
// Rust 对应声明 (rusl)
// 注：内部辅助函数，仅在缺少 SYS_wait4 时编译
unsafe fn __emulate_wait4(
    pid: c_int,
    status: *mut c_int,
    options: c_int,
    kru: *mut c_void,
    cp: c_int,
) -> c_long;
```

**[Visibility]: Internal (不导出)** — 仅在 `syscall.h` 中通过宏 `__sys_wait4` / `__sys_wait4_cp` 间接调用，是 musl 的内部系统调用兼容层。POSIX/C 标准未定义。

**编译条件**: 仅在 `#ifndef SYS_wait4` 成立时（即目标架构不提供原生 `wait4` 系统调用）才编译此文件。

---

### 前置条件

1. `f` 指向的 FILE 流已通过 `shlim` 设置了读取限制（或在 string 模式下 `shlim` 可为 0）
2. `pid` 必须符合 POSIX wait4 的语义：`< -1`（进程组）、`-1`（任意子进程）、`0`（同进程组）、`> 0`（特定进程）
3. `status` 可以为 `NULL`（调用者不关心退出状态）
4. `kru` 为 `struct rusage*`，可以为 `NULL`
5. `cp` 为 `0`（不可取消）或 `1`（可取消点），控制是否通过 `__syscall_cp` 调用

不变量：
- `info.si_pid` 在 `SYS_waitid` 成功后被内核填入（0 表示无匹配子进程）

### 后置条件

**Case 1 — 系统调用成功 (r >= 0)**:
- 返回子进程 `pid`（即 `info.si_pid`）
- 若 `info.si_pid != 0 && status != NULL`，则 `*status` 被设为 POSIX wait 状态编码：
  - `CLD_CONTINUED` → `*status = 0xffff`（即 `WIFCONTINUED` 标记）
  - `CLD_DUMPED` → `*status = (si_status & 0x7f) | 0x80`（即 `WCOREDUMP` + 信号低 7 位）
  - `CLD_EXITED` → `*status = (si_status & 0xff) << 8`（即 `WEXITSTATUS` 编码位置）
  - `CLD_KILLED` → `*status = si_status & 0x7f`（仅信号低 7 位，无 core dump 标记）
  - `CLD_STOPPED` / `CLD_TRAPPED` → `*status = (si_status << 8) + 0x7f`（高字节保留 `PTRACE_EVENT_*` 值）
- 若 `info.si_pid == 0`，返回 0（无匹配子进程）

**Case 2 — 系统调用失败 (r < 0)**:
- 直接返回负值 errno（即 `__syscall` / `__syscall_cp` 的错误返回值）
- `*status` 不被修改

### 系统算法（System Algorithm）

该函数实现了 **pid → idtype_t 映射 + siginfo_t → wait status 转换** 的两步模拟：

1. **PID 到等待类型映射**:
   - `pid < -1` → `P_PGID`，取绝对值作为进程组 ID
   - `pid == -1` → `P_ALL`，等待任意子进程
   - `pid == 0` → `P_PGID`，等待同一进程组
   - `pid > 0` → `P_PID`，等待特定子进程

2. **waitid 系统调用**: 始终附加 `WEXITED` 标志（`wait4` 语义要求包含已终止子进程），`options` 直接透传。

3. **siginfo_t → wait status 编码**: 将 `si_code`（子进程状态原因）和 `si_status` 编码为传统 UNIX wait 状态字，使调用者可通过 `WIFEXITED`、`WEXITSTATUS` 等宏解析。

### 不变量（Invariants）

- `info.si_pid` 在进入 `SYS_waitid` 调用前被初始化为 `0`，确保系统调用失败时不被误读为有效 pid
- `WEXITED` 标志始终被 OR 进 options，保证 `wait4` 语义完整性