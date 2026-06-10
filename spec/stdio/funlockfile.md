# funlockfile.c 规约

> musl libc 文件流解锁实现。支持递归锁的递减与最终的锁释放。

---

## 依赖图

```
funlockfile
  ├─> __unlist_locked_file      (see ftrylockfile.c spec)
  │     └─> __pthread_self      (see pthread_self.c spec)
  └─> __unlockfile              (see __lockfile.c spec)
        ├─> a_swap              (see atomic.h)
        └─> __wake              (see futex.h)
```

---

## 函数规约

### 1. funlockfile

```c
void funlockfile(FILE *f);
```

[Visibility]: User — POSIX 标准函数，声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

释放文件流 `f` 的互斥锁。支持递归解锁：当 `lockcount > 1` 时仅递减计数；当 `lockcount == 1`（即将完全释放锁）时从线程的持有锁链表中移除 `f`、清零计数，并调用 `__unlockfile` 释放底层锁（如有等待者则唤醒）。

`funlockfile` 与 `flockfile`/`ftrylockfile` 配合使用。每个成功获取必须对应一次释放。当用户显式调用这些函数时，应确保配对。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`
- 当前线程持有 `f` 的锁（`lockcount >= 1`）
- 调用方是锁的合法持有者

#### 后置条件

**Case 1: 递归释放（lockcount > 1）**
- `f->lockcount--`（递减递归计数）
- 锁保持被当前线程持有
- `f` 仍在线程的持有锁链表中

**Case 2: 完全释放（lockcount == 1）**
- `__unlist_locked_file(f)` 将 `f` 从线程的持有锁链表中移除
- `f->lockcount = 0`
- `__unlockfile(f)` 释放底层锁：`a_swap(&f->lock, 0)`，若有等待者（`MAYBE_WAITERS`）则 `__wake` 唤醒一个
- 函数返回后，其他线程可获取该锁

#### 系统算法

```
funlockfile(f):
  if f->lockcount == 1:           // 最后一次释放
    __unlist_locked_file(f)       // 从线程锁链表移除
    f->lockcount = 0              // 清零计数
    __unlockfile(f)               // 释放底层锁（原子交换+可能唤醒）
  else:                           // 递归释放
    f->lockcount--                // 仅递减计数
```

#### 不变量

- 每次成功获取（`flockfile` 或 `ftrylockfile`）都有对应的释放
- `lockcount == 0` 时锁对任何线程可用
- `lockcount > 0` 时锁由当前线程持有

#### 依赖

- `__unlist_locked_file` — 从线程持有锁链表移除 FILE（定义于 `src/stdio/ftrylockfile.c`，见 `ftrylockfile.c` spec）
- `__unlockfile` — 释放底层锁并唤醒等待者（定义于 `src/stdio/__lockfile.c`，见 `__lockfile.c` spec）
