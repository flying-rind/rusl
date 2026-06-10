# flockfile.c 规约

> musl libc 文件流阻塞锁定实现。获取文件流的递归锁，若不能立即获取则阻塞等待。

---

## 依赖图

```
flockfile
  ├─> ftrylockfile              (see ftrylockfile.c spec)
  │     ├─> __pthread_self      (see pthread_self.c spec)
  │     ├─> a_cas               (see atomic.h)
  │     └─> __register_locked_file (see ftrylockfile.c spec)
  ├─> __lockfile                (see __lockfile.c spec)
  │     ├─> a_cas               (see atomic.h)
  │     ├─> __futexwait         (see futex.h)
  │     └─> __pthread_self      (see pthread_self.c spec)
  ├─> __register_locked_file    (see ftrylockfile.c spec)
  └─> __pthread_self            (see pthread_self.c spec)
```

---

## 函数规约

### 1. flockfile

```c
void flockfile(FILE *f);
```

[Visibility]: User — POSIX 标准函数，声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

获取文件流 `f` 的互斥锁，必要时阻塞等待。先尝试非阻塞获取（`ftrylockfile`），若失败则降级为阻塞式获取（`__lockfile`），之后再注册到线程的持有锁链表。

此函数组合了两个路径：
1. **快速路径**: `ftrylockfile` 成功 → 直接返回
2. **慢速路径**: `ftrylockfile` 失败 → `__lockfile` 阻塞等待 → 注册到链表

#### 前置条件

- `f`: 非 NULL 的 `FILE*`
- `__pthread_self()` 返回有效线程控制块

#### 后置条件

**Case 1: 立即获取成功**
- `ftrylockfile(f)` 返回 `0`
- 函数直接返回

**Case 2: 阻塞后获取成功**
- `ftrylockfile(f)` 返回 `-1`（锁不可立即获取）
- `__lockfile(f)` 阻塞直到获取锁并返回
- `__register_locked_file(f, __pthread_self())` 将 `f` 注册到线程的持有锁链表
- 函数返回

#### 系统算法

```
flockfile(f):
  if !ftrylockfile(f):            // 1. 快速路径：尝试非阻塞获取
    return                        //    成功，已注册
  __lockfile(f)                   // 2. 慢速路径：阻塞等待获取
  __register_locked_file(f, __pthread_self())  // 3. 注册到线程链表
```

#### 不变量

- 成功返回时当前线程持有 `f` 的锁
- `f` 被注册在当前线程的 `stdio_locks` 链表中
- 同一线程可递归调用 `flockfile`（通过 `ftrylockfile` 的递归路径或 `__lockfile` 的等待）

#### 依赖

- `ftrylockfile` — 非阻塞锁尝试（定义于 `src/stdio/ftrylockfile.c`，见 `ftrylockfile.c` spec）
- `__lockfile` — 阻塞锁获取（定义于 `src/stdio/__lockfile.c`，见 `__lockfile.c` spec）
- `__register_locked_file` — 注册到持有锁链表（定义于 `src/stdio/ftrylockfile.c`）
- `__pthread_self` — 获取当前线程控制块（`src/thread/pthread_self.c`）
