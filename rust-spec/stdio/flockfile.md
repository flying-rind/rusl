# flockfile 函数规约

## 复杂度分级: Level 2

> musl libc 文件流阻塞锁定实现。获取文件流的递归锁，若不能立即获取则阻塞等待。

---

## 函数接口

```rust
use crate::internal::FILE;

extern "C" fn flockfile(f: *mut FILE);
```

[Visibility]: `flockfile` 是 POSIX 标准函数，声明于 `<stdio.h>`，对外导出。

Rust 侧实现策略：
- 内部两条路径：快速路径（`ftrylockfile` 成功）和慢速路径（`__lockfile` 阻塞等待）
- `ftrylockfile`：非阻塞获取尝试
- `__lockfile`：阻塞式锁获取（使用 futex 或平台相关原语）
- `__register_locked_file`：将 FILE 注册到线程的持有锁链表
- `__pthread_self`：获取当前线程控制块

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 非 NULL 的 `FILE*`
- `__pthread_self()` 返回有效线程控制块

**[Post-condition]:**
- **Case 1: 立即获取成功**
  - `ftrylockfile(f)` 返回 `0`
  - 函数直接返回

- **Case 2: 阻塞后获取成功**
  - `ftrylockfile(f)` 返回 `-1`（锁不可立即获取）
  - `__lockfile(f)` 阻塞直到获取锁并返回
  - `__register_locked_file(f, __pthread_self())` 将 `f` 注册到线程的持有锁链表
  - 函数返回

**[Error Behavior]:**
- 始终成功返回（`void` 函数），通过阻塞保证最终获取锁

---

### 不变量

**[Invariant]:**
- 成功返回时当前线程持有 `f` 的锁
- `f` 被注册在当前线程的 `stdio_locks` 链表中
- 同一线程可递归调用 `flockfile`（通过 `ftrylockfile` 的递归路径或 `__lockfile` 的等待）

---

### 意图

获取文件流 `f` 的互斥锁，必要时阻塞等待。先尝试非阻塞获取（`ftrylockfile`），若失败则降级为阻塞式获取（`__lockfile`），之后再注册到线程的持有锁链表。

此函数组合了两个路径：
1. **快速路径**: `ftrylockfile` 成功直接返回
2. **慢速路径**: `ftrylockfile` 失败 `__lockfile` 阻塞等待 注册到链表

---

### 系统算法

```
flockfile(f):
  if !ftrylockfile(f):           // 1. 快速路径：尝试非阻塞获取
    return                       //    成功，已注册
  __lockfile(f)                  // 2. 慢速路径：阻塞等待获取
  __register_locked_file(f, __pthread_self()) // 3. 注册到线程链表
```

时间复杂度：快速路径 O(1)，慢速路径取决于锁竞争程度。

---

## 依赖图

```
flockfile
  ├── ftrylockfile              (see ftrylockfile.c spec)
  │     ├── __pthread_self      (see pthread_self.c spec)
  │     ├── a_cas               (see atomic.h)
  │     └── __register_locked_file (see ftrylockfile.c spec)
  ├── __lockfile                (see __lockfile.c spec)
  │     ├── a_cas               (see atomic.h)
  │     ├── __futexwait         (see futex.h)
  │     └── __pthread_self      (see pthread_self.c spec)
  ├── __register_locked_file    (see ftrylockfile.c spec)
  └── __pthread_self            (see pthread_self.c spec)
```

---

## [RELY]

- `ftrylockfile` — 非阻塞锁尝试 (见 `ftrylockfile.md`)
- `__lockfile` — 阻塞锁获取 (见 `__lockfile.md`)
- `__register_locked_file` — 注册到持有锁链表 (定义于 `ftrylockfile.c`)
- `__pthread_self` — 获取当前线程控制块 (见 `pthread_self.md`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn flockfile(f: *mut FILE);`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 POSIX flockfile 语义。
