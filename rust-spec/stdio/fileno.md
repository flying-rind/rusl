# fileno 函数规约

## 复杂度分级: Level 1

> musl libc 获取文件流底层文件描述符的 Rust 实现。提供 `fileno` 和 POSIX 免锁扩展 `fileno_unlocked`。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性

unsafe extern "C" fn fileno(f: *mut FILE) -> c_int;

// weak_alias: fileno_unlocked 是 fileno 的弱别名，共享同一实现
unsafe extern "C" fn fileno_unlocked(f: *mut FILE) -> c_int;
```

[Visibility]:
- `fileno` — **User**，POSIX 标准函数，声明于 `<stdio.h>`（需 `_POSIX_C_SOURCE >= 200112L`），用户程序可直接调用
- `fileno_unlocked` — **User**，POSIX 扩展函数，声明于 `<stdio.h>`，用户程序可直接调用

---

## 前置/后置条件

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，指向已正确初始化的 `FILE` 结构体

**[Post-condition]:**

**Case 1: 成功 — 流有关联的有效文件描述符**
- `f.fd >= 0`
- 返回 `f.fd`（非负整数，即底层文件描述符）
- `errno` 保持不变

**Case 2: 失败 — 流未关联有效文件描述符**
- `f.fd < 0`（如某些内存流场景）
- `errno` 设置为 `EBADF`（错误的文件描述符）
- 返回 `-1`

**[Error Behavior]:**
- 当且仅当 `f.fd < 0` 时设置 `errno = EBADF` 并返回 `-1`
- 不会因其他原因失败

---

## 不变量

**[Invariant]:**
- 仅读取 `f.fd` 字段，不修改 `FILE` 结构体的任何状态
- `fileno` 和 `fileno_unlocked` 行为完全一致，返回相同结果
- 操作在锁保护下原子执行，保证线程安全
- 返回的文件描述符值在流关闭前保持有效

---

## 意图

获取与文件流 `f` 关联的底层文件描述符。可用于在需要文件描述符的系统调用（如 `fcntl`、`fstat`、`ioctl` 等）中直接操作底层文件。

`FILE` 结构体中 `fd` 字段由 `__fdopen` 或 `fopen` 在流初始化时设置。有效的 `fd` 值 >= `0`；`< 0` 表示流未与有效文件描述符关联。

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，`fd` 字段（类型 `c_int`）与原 C 布局完全一致
- `FLOCK`/`FUNLOCK` 内部通过调用 `__lockfile`/`__unlockfile` 实现，或使用 Rust 的安全锁抽象包装 `FILE` 的锁字段
- `EBADF` 常量值（`9`）定义于 `rusl-errno` 或当前平台的 `<errno.h>` 对应模块
- `errno` 通过 `__errno_location()` 获取地址后写入
- 弱别名 `fileno_unlocked` 通过 `#[no_mangle]` + 相同函数体实现，保证链接时解析为同一地址

## 系统算法

```
fileno(f: *mut FILE) -> c_int:
  FLOCK(f)                          // 获取 f 的互斥锁
  fd = (*f).fd                      // 读取内部文件描述符字段
  FUNLOCK(f)                        // 释放 f 的互斥锁
  if fd < 0:
    *__errno_location() = EBADF     // 设置 errno = EBADF
    return -1
  return fd

fileno_unlocked(f: *mut FILE) -> c_int:
  同 fileno() 的函数体
```

时间复杂度 O(1)。

---

## 依赖图

```
fileno
  ├─> FLOCK / __lockfile        (see __lockfile spec)
  ├─> FUNLOCK / __unlockfile    (see __lockfile spec)
  └─> __errno_location          (see __errno_location spec)

fileno_unlocked = weak_alias(fileno)
```

---

## [RELY]

- `FLOCK` / `FUNLOCK` — 流锁定/解锁，内部依赖 `__lockfile`/`__unlockfile`（见 `__lockfile` spec）
- `FILE` 结构体定义 — `fd` 字段布局（见 `stdio_impl` 模块）
- `__errno_location` — 获取 errno 地址（见 `__errno_location` spec）
- `EBADF` — 错误码常量（值 `9`），定义于 errno 模块

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn fileno(f: *mut FILE) -> c_int;
unsafe extern "C" fn fileno_unlocked(f: *mut FILE) -> c_int;
```

本模块保证对外提供上述两个 ABI 兼容的函数符号：
- `fileno`: 线程安全版本，符合 POSIX 标准，加锁获取底层文件描述符
- `fileno_unlocked`: 弱别名，行为与 `fileno` 完全一致

成功时返回非负文件描述符（`>= 0`）且不修改 errno；失败时设置 `errno = EBADF` 并返回 `-1`。
