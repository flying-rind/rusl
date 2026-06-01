# syscall.h 规约

> **来源文件**: `musl/src/internal/syscall.h`
> **复杂度层级**: Level 3 — 高度优化设计（宏调度系统 + 平台兼容层）
> **依赖图**:
> ```
> features.h (提供 hidden/weak 属性)
>   -> syscall_arch.h (架构相关 __syscall0~6 内联汇编实现)
>     -> __syscall_ret() — 返回值 errno 转换
>     -> __syscall_cp() — 带取消点的系统调用
>       -> __alt_socketcall() — socket 调用统一调度
>         -> __sys_open() 系列 — open/openat 统一
>           -> __sys_pause() — pause/ppoll 统一
>             -> syscall() / syscall_cp() — 高层宏
>               -> sys_open / sys_open_cp / sys_pause 等便捷宏
>                 -> 外部可见: __procfdname(), __vdsosym(), __emulate_wait4()
> ```

---

## 概述

`syscall.h` 是 musl libc 系统调用层的核心头文件。它定义了统一的系统调用调度机制，将各架构的原始 syscall 指令（来自 `syscall_arch.h`）包装为类型安全的 C 宏和函数，并提供以下关键服务：
1. **参数类型统一** (`__scc` → `syscall_arg_t`)：所有 syscall 参数统一转为 `long` 类型，避免 32/64 位架构差异
2. **可变参数调度** (`__SYSCALL_DISP`)：根据参数个数自动选择 `__syscall1` ~ `__syscall7`
3. **返回值标准化** (`__syscall_ret`)：将内核返回的负数 errno 转为 `-1` 并设置 `errno`
4. **取消点支持** (`__syscall_cp` 系列)：在系统调用前后检查线程取消标志
5. **平台兼容修复** (`#ifdef SYS_*`)：处理 32/64 位 ABI 差异（如 `stat64`、`fstat64` 等）、time64 过渡、socketcall 多路复用

**不变量 (Invariants)**：
- **I1**: 所有经 `__syscall_ret` 处理的返回值保证：若调用成功返回非负值，若失败则 `errno` 被设置为正值且函数返回 `-1`。
- **I2**: 经 `__syscall_cp` 系列的调用在进入内核前检查取消点；若线程已被取消，系统调用**绝不执行**。
- **I3**: 所有 syscall 宏展开后参数均经 `__scc()` 转换为 `(long)`，保证寄存器宽度一致性。

---

## 类型定义

### `syscall_arg_t`

```c
typedef long syscall_arg_t;
```

[Visibility]: Internal — musl 内部类型，POSIX/C 标准未定义

系统调用参数的统一类型。在 64 位架构上 `long` 为 64 位；在 32 位架构上 `long` 为 32 位。所有 `__syscallN()` 接受 `syscall_arg_t` 型参数，避免隐式类型转换导致寄存器赋值错误。

---

## 核心函数声明

### `long __syscall_ret(unsigned long)`

```c
long __syscall_ret(unsigned long);
```

[Visibility]: Internal — musl 内部系统调用返回值处理器

**意图 (Intent)**：
将 Linux 内核系统调用的原始返回值（负数 = -errno 表示错误，非负 = 成功）转换为符合 C 库约定的返回值（-1 表示错误并设置 errno）。

**前置条件 (Preconditions)**：
- **P1**: `r` 是某次 Linux 系统调用的原始返回值（即 `__syscallN` 的返回值）。
- **P2**: `r` 为 `unsigned long` 类型，携带内核返回的完整 64/32 位值。

**后置条件 (Postconditions)**：
- **Case 1 (`r > -4096UL`，即错误)**：
  - 返回值 = `-1`
  - `errno = -(int)r`（将负数 errno 转为正数）
- **Case 2 (`r <= -4096UL`，即成功或不表示错误的负值)**：
  - 返回值 = `(long)r`（原样返回，可能为负的大地址如 `mmap` 返回值）

**系统算法 (System Algorithm)**：
Linux 系统调用错误返回值范围约定为 `[-4095, -1]`（即 `> (unsigned long)-4096`）。实现伪代码：
```c
if (r > -4096UL) { errno = -(int)r; return -1; }
return (long)r;
```

---

### `long __syscall_cp(syscall_arg_t, syscall_arg_t, syscall_arg_t, syscall_arg_t, syscall_arg_t, syscall_arg_t, syscall_arg_t)`

```c
long __syscall_cp(syscall_arg_t, syscall_arg_t, syscall_arg_t,
                  syscall_arg_t, syscall_arg_t, syscall_arg_t, syscall_arg_t);
```

[Visibility]: Internal — musl 内部带取消点的系统调用

**意图 (Intent)**：
执行系统调用前检查线程取消标志，若线程已标记取消则立即执行取消操作而不进入内核。参数传递方式与 `__syscall6` 相同，前 6 个为 syscall 参数，第 7 个为 syscall 号。

**前置条件 (Preconditions)**：
- **P1**: 调用线程必须处于可取消状态（caller context supports cancellation）。
- **P2**: 第 7 个参数为有效的 Linux syscall 号。

**后置条件 (Postconditions)**：
- **Case 1（线程已被取消）**：
  - 系统调用**不执行**。
  - 线程执行 `__testcancel()` → `pthread_exit(PTHREAD_CANCELED)`，函数不返回。
- **Case 2（线程未被取消）**：
  - 执行实际的系统调用，返回值经 `__syscall_ret` 处理后返回。

**系统算法 (System Algorithm)**：
采用 Linux `setjmp/longjmp` 风格的取消点实现。在进入 syscall 前设置取消点标记，使得信号处理器可在 syscall 被 `EINTR` 中断时触发取消。

---

### `long __alt_socketcall(int, int, int, syscall_arg_t, syscall_arg_t, syscall_arg_t, syscall_arg_t, syscall_arg_t, syscall_arg_t)`

```c
static inline long __alt_socketcall(int sys, int sock, int cp,
    syscall_arg_t a, syscall_arg_t b, syscall_arg_t c,
    syscall_arg_t d, syscall_arg_t e, syscall_arg_t f);
```

[Visibility]: Internal — musl 内部 socket 系统调用统一调度器（static inline）

**意图 (Intent)**：
在支持独立 socket 系统调用的新架构上直接调用；在仅支持 `socketcall()` 多路复用的旧架构上退化为 `socketcall(SYS_socketcall, ...)` 方式。

**前置条件 (Preconditions)**：
- **P1**: `sys` 为有效的 socket SYS_ 号（如 `SYS_connect`）。
- **P2**: `sock` 为对应的 `__SC_*` 子调用号（如 `__SC_connect = 3`）。
- **P3**: `cp` 为 0（不带取消点）或 1（带取消点）。

**后置条件 (Postconditions)**：
- **Q1**: 返回值是经 `__syscall_ret` 处理的系统调用结果（或 `__syscall_cp` 处理）。
- **Q2**: 若直接调用返回 `-ENOSYS`（内核不支持独立 socket syscall）且 `SYS_socketcall` 已定义，则退化为 `socketcall(SYS_socketcall, sock, ...)` 再试一次。

**系统算法 (System Algorithm)**：
```
1. if (cp) r = __syscall_cp(sys, a, b, c, d, e, f)
   else    r = __syscall(sys, a, b, c, d, e, f)
2. if r != -ENOSYS → return r
3. ifdef SYS_socketcall:
     if (cp) r = __syscall_cp(SYS_socketcall, sock, [a..f])
     else    r = __syscall(SYS_socketcall, sock, [a..f])
4. return r
```

---

### `void __procfdname(char [static 15+3*sizeof(int)], unsigned)`

```c
void __procfdname(char __buf[static 15+3*sizeof(int)], unsigned);
```

[Visibility]: Internal — musl 内部辅助函数

**意图 (Intent)**：
将文件描述符号转换为对应的 `/proc/self/fd/%u` 路径字符串，用于 `fchmod`、`fstat` 等操作无 `fd` 版本时需要打开 `/proc/self/fd/N` 的场景。

**前置条件 (Preconditions)**：
- **P1**: `buf` 指向至少有 `15 + 3*sizeof(int)` 字节的缓冲区。
- **P2**: `fd` 为有效的文件描述符编号。

**后置条件 (Postconditions)**：
- **Q1**: `buf[0..]` 中包含以 `\0` 结尾的路径字符串 `"/proc/self/fd/<fd>"`，其中 `<fd>` 为 `fd` 参数的十进制表示。

---

### `void *__vdsosym(const char *, const char *)`

```c
void *__vdsosym(const char *, const char *);
```

[Visibility]: Internal — musl 内部 VDSO 符号查找

**意图 (Intent)**：
从 Linux vDSO（virtual dynamic shared object）中按名称和版本查找函数指针，用于快速获取 `clock_gettime` 等高频系统调用的用户态实现。

**前置条件 (Preconditions)**：
- **P1**: `name` 非空，为有效的 ELF 符号名（如 `"__vdso_clock_gettime"`）。
- **P2**: `ver` 非空，为有效的 ELF 版本字符串（如 `"LINUX_2.6"`）。

**后置条件 (Postconditions)**：
- **Case 1（成功）**：返回 vDSO 中对应符号的地址（函数指针）。
- **Case 2（失败）**：返回 `NULL`（vDSO 不可用或符号不存在）。

---

## 系统调用调度宏体系

以下宏构成统一的可变参数系统调用调度系统，均为 **Internal** 可见性。

### 参数数量计数宏

```c
#define __SYSCALL_NARGS_X(a,b,c,d,e,f,g,h,n,...) n
#define __SYSCALL_NARGS(...) __SYSCALL_NARGS_X(__VA_ARGS__,7,6,5,4,3,2,1,0,)
```

**意图**: 编译期计参数数量。通过将参数列表与降序数字序列拼接，取第 9 个参数即为参数个数。

### 标记拼接宏

```c
#define __SYSCALL_CONCAT_X(a,b) a##b
#define __SYSCALL_CONCAT(a,b) __SYSCALL_CONCAT_X(a,b)
```

**意图**: 将 syscall 族名前缀与参数个数拼接，如 `__syscall` + `3` → `__syscall3`。

### 调度宏

```c
#define __SYSCALL_DISP(b,...) __SYSCALL_CONCAT(b,__SYSCALL_NARGS(__VA_ARGS__))(__VA_ARGS__)
```

**意图**: 核心调度器。`__SYSCALL_DISP(__syscall, SYS_read, fd, buf, len)` 展开为 `__syscall3(SYS_read, fd, buf, len)`。

### 顶层调用宏

```c
#define __syscall(...) __SYSCALL_DISP(__syscall,__VA_ARGS__)
#define syscall(...) __syscall_ret(__syscall(__VA_ARGS__))
```

**意图**:
- `__syscall(...)` — 执行原始系统调用，返回未处理的 `unsigned long` 值
- `syscall(...)` — 执行系统调用并自动进行 errno 转换

### 带取消点的宏

```c
#define __syscall_cp(...) __SYSCALL_DISP(__syscall_cp,__VA_ARGS__)
#define syscall_cp(...) __syscall_ret(__syscall_cp(__VA_ARGS__))
```

**意图**: 与上述对应，但在系统调用前检查线程取消标志。

### 显式参数数量宏

```c
#define __syscall1(n,a) __syscall1(n,__scc(a))
#define __syscall2(n,a,b) __syscall2(n,__scc(a),__scc(b))
...
#define __syscall7(n,a,b,c,d,e,f,g) __syscall7(n,__scc(a),__scc(b),__scc(c),__scc(d),__scc(e),__scc(f),__scc(g))
```

**意图**: 类型安全包装。在传递参数给架构级 `__syscallN` 内联函数前，强制将每个参数转为 `long` 类型。

### 取消点参数宏

```c
#define __syscall_cp0(n) (__syscall_cp)(n,0,0,0,0,0,0)
#define __syscall_cp1(n,a) (__syscall_cp)(n,__scc(a),0,0,0,0,0)
...
#define __syscall_cp6(n,a,b,c,d,e,f) (__syscall_cp)(n,__scc(a),__scc(b),__scc(c),__scc(d),__scc(e),__scc(f))
```

**意图**: 补零填充，确保所有未使用的参数位置被显式置 0。

---

## Socket 调用子系统

### socketcall 调度宏

```c
#define socketcall(nm,a,b,c,d,e,f) __syscall_ret(__socketcall(nm,a,b,c,d,e,f))
#define socketcall_cp(nm,a,b,c,d,e,f) __syscall_ret(__socketcall_cp(nm,a,b,c,d,e,f))
```

其中 `__socketcall` 和 `__socketcall_cp` 展开为对 `__alt_socketcall(SYS_##nm, __SC_##nm, cp, ...)` 的调用。

### Socket 子调用号常量

```c
#define __SC_socket      1
#define __SC_bind        2
#define __SC_connect     3
#define __SC_listen      4
#define __SC_accept      5
#define __SC_getsockname 6
#define __SC_getpeername 7
#define __SC_socketpair  8
#define __SC_send        9
#define __SC_recv        10
#define __SC_sendto      11
#define __SC_recvfrom    12
#define __SC_shutdown    13
#define __SC_setsockopt  14
#define __SC_getsockopt  15
#define __SC_sendmsg     16
#define __SC_recvmsg     17
#define __SC_accept4     18
#define __SC_recvmmsg    19
#define __SC_sendmmsg    20
```

**意图**: 对应 Linux `SYS_socketcall` 的第二个参数（call 子编号）。

---

## 文件操作统一接口

### open/openat 统一

```c
#define __sys_open(...) __SYSCALL_DISP(__sys_open,,__VA_ARGS__)
#define sys_open(...) __syscall_ret(__sys_open(__VA_ARGS__))
```

**意图**: 在有 `SYS_open` 的内核上使用 `open()`；在没有该 syscall 的新内核上使用 `openat(AT_FDCWD, ...)`。

### pause/ppoll 统一

```c
#define __sys_pause() __syscall(SYS_pause)  // 或退化为 ppoll(0,0,0,0)
```

### wait4 可选模拟

```c
#define __sys_wait4(a,b,c,d)   __syscall(SYS_wait4,a,b,c,d)
#define __sys_wait4_cp(a,b,c,d) __syscall_cp(SYS_wait4,a,b,c,d)
```

在无 `SYS_wait4` 的架构上退化为调用 `__emulate_wait4()`（hidden 函数，跨文件依赖）。

---

## 平台兼容性修复

以下 `#ifdef SYS_*` 块处理不同 Linux 内核版本间的 ABI 差异：

### 32位 UID/GID 修复
- 条件编译块 `#ifdef SYS_getuid32`: 将 16 位 UID syscall 重定向为 32 位版本（`SYS_getuid → SYS_getuid32`）

### 大文件支持修复
- `SYS_fcntl → SYS_fcntl64`、`SYS_getdents → SYS_getdents64`
- `SYS_stat → SYS_stat64`、`SYS_fstat → SYS_fstat64`、`SYS_lstat → SYS_lstat64`
- `SYS_ftruncate → SYS_ftruncate64`、`SYS_truncate → SYS_truncate64`
- `SYS_pread → SYS_pread64`、`SYS_pwrite → SYS_pwrite64`

### Time64 修复
- `SYS_clock_gettime → SYS_clock_gettime64`、`SYS_clock_settime → SYS_clock_settime64`
- `SYS_timer_gettime → SYS_timer_gettime64`、`SYS_timer_settime → SYS_timer_settime64`
- `SYS_futex → SYS_futex_time64` 等

### Time32 修复（反向兼容）
- 若定义 `SYS_clock_gettime32`：`SYS_clock_gettime → SYS_clock_gettime32`（用于 32 位 time_t 用户态在 64 位内核）

### Socket 超时选项常量
```c
#define SO_RCVTIMEO_OLD  20
#define SO_SNDTIMEO_OLD  21
```
用于区分新旧内核的 socket 超时选项编号。

### Socket 时间戳常量
```c
#define SO_TIMESTAMP_OLD    29
#define SO_TIMESTAMPNS_OLD  35
#define SO_TIMESTAMPING_OLD 37
```
以及对应的 `SCM_TIMESTAMP_OLD` 等，用于兼容旧内核时间戳格式。

### accept4 降级
```c
#ifndef SYS_accept
#define SYS_accept SYS_accept4
#endif
```
在仅有 `accept4` 的新内核上，将 `accept` 映射为 `accept4(fd, addr, addrlen, 0)`。

---

## 其他常量定义

| 常量 | 值 | 意义 |
|------|-----|------|
| `SYSCALL_RLIM_INFINITY` | `~0ULL` | rlimit 无限制值 |
| `SYSCALL_MMAP2_UNIT` | `4096ULL` | mmap2 偏移单位（页大小） |

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `__syscall0` ~ `__syscall6` | `syscall_arch.h`（架构相关） | 外部依赖，由具体架构的 ABI 约定实现 |
| `__syscall_ret()` | `src/internal/syscall_ret.c` | 跨文件实现，本头文件仅声明 |
| `__syscall_cp()` | `src/thread/` 相关文件 | 跨文件实现，本头文件仅声明 |
| `__emulate_wait4()` | `src/process/` 相关文件 | 跨文件实现，条件编译时可见 |
| `__procfdname()` | `src/internal/procfdname.c` | 跨文件实现 |
| `__vdsosym()` | `src/internal/vdso.c` | 跨文件实现 |
| `hidden`, `weak` 属性 | `features.h` | 跨文件依赖（`src/include/features.h`），提供编译器属性宏 |
| `SYS_*` 常量 | `<sys/syscall.h>`（Linux 内核头文件） | 系统级外部依赖 |

---

## 实现指南 (rusl/Rust)

- `syscall_arg_t` → `type SyscallArg = isize;`（或 `i64` 统一处理）
- `__syscall_ret()` → `fn syscall_ret(r: usize) -> isize`：`if r > -4096usize { set_errno(-(r as isize)); -1 } else { r as isize }`
- `__syscall_cp()` → 在调用 syscall 前通过原子变量检查取消标志
- 可变参数调度 → 使用 Rust 宏（`macro_rules!`）实现类似的参数计数 + 拼接机制
- 平台兼容 `#ifdef` → 使用 Rust 的 `#[cfg(target_arch = "...")]` 和 `cfg` 属性
- `__alt_socketcall()` → 普通函数（不需 static inline），利用 Rust 内联优化
- vDSO 查找 → 解析 `auxv` 获取 vDSO 基址，手动 ELF 符号查找