# futex.h 规约

## 概述

`futex.h` 定义了 Linux `futex(2)` 系统调用的操作码宏常量。futex (fast userspace mutex) 是 Linux 内核提供的一种轻量级同步原语，musl 使用它实现线程锁、信号量、屏障等所有 pthread 同步机制。本头文件不依赖任何其他 musl 内部头文件，仅定义编译期常量。

## 依赖图

```
(无 musl 内部依赖)
外部依赖: <linux/futex.h> (内核接口，语义等价)
```

---

## 符号规约

### FUTEX_WAIT
```c
#define FUTEX_WAIT 0
```
```
// Rust 等价定义
pub const FUTEX_WAIT: i32 = 0;
```

[Visibility]: Internal — Linux futex 系统调用操作码，musl 内部使用，非 POSIX/C 标准定义

**Intent**: 对 futex 字(32-bit 整数)进行等待操作。调用线程挂起直到 futex 字的值不等于预期值 `val`，或被 `FUTEX_WAKE` 唤醒。

**Pre/Post 条件**: 不适用（宏常量，无运行时行为）。

---

### FUTEX_WAKE
```c
#define FUTEX_WAKE 1
```
```
pub const FUTEX_WAKE: i32 = 1;
```

[Visibility]: Internal — Linux futex 系统调用操作码

**Intent**: 唤醒最多 `val` 个在 futex 字上等待的线程。

---

### FUTEX_FD
```c
#define FUTEX_FD 2
```
```
pub const FUTEX_FD: i32 = 2;
```

[Visibility]: Internal — Linux futex 系统调用操作码（已废弃，musl 保留以保持兼容）

**Intent**: 将 futex 关联到文件描述符，用于异步通知。此操作已从现代 Linux 内核中移除。

---

### FUTEX_REQUEUE
```c
#define FUTEX_REQUEUE 3
```
```
pub const FUTEX_REQUEUE: i32 = 3;
```

[Visibility]: Internal — Linux futex 系统调用操作码

**Intent**: 将等待者从主 futex 迁移到另一个 futex。用于实现 `pthread_cond_broadcast` 等条件变量操作，避免惊群效应。

---

### FUTEX_CMP_REQUEUE
```c
#define FUTEX_CMP_REQUEUE 4
```
```
pub const FUTEX_CMP_REQUEUE: i32 = 4;
```

[Visibility]: Internal — Linux futex 系统调用操作码

**Intent**: 与 `FUTEX_REQUEUE` 类似，但额外检查主 futex 字的值是否等于预期值 `val3`。此检查与迁移操作原子化执行，防止竞态条件。

**Invariant**: `FUTEX_CMP_REQUEUE` 必须与 `FUTEX_REQUEUE` 配合使用——使用 `CMP_REQUEUE` 检查条件后，若匹配则执行 `REQUEUE` 语义。

---

### FUTEX_WAKE_OP
```c
#define FUTEX_WAKE_OP 5
```
```
pub const FUTEX_WAKE_OP: i32 = 5;
```

[Visibility]: Internal — Linux futex 系统调用操作码

**Intent**: 原子化地修改 futex 字并唤醒等待者。避免在唤醒者和被唤醒者之间引入额外的系统调用。

---

### FUTEX_LOCK_PI
```c
#define FUTEX_LOCK_PI 6
```
```
pub const FUTEX_LOCK_PI: i32 = 6;
```

[Visibility]: Internal — Linux futex 系统调用操作码

**Intent**: 带优先级继承的 futex 加锁操作。用于实现 `PTHREAD_PRIO_INHERIT` 互斥锁，防止优先级反转。

---

### FUTEX_UNLOCK_PI
```c
#define FUTEX_UNLOCK_PI 7
```
```
pub const FUTEX_UNLOCK_PI: i32 = 7;
```

[Visibility]: Internal — Linux futex 系统调用操作码

**Intent**: 带优先级继承的 futex 解锁操作。

---

### FUTEX_TRYLOCK_PI
```c
#define FUTEX_TRYLOCK_PI 8
```
```
pub const FUTEX_TRYLOCK_PI: i32 = 8;
```

[Visibility]: Internal — Linux futex 系统调用操作码

**Intent**: 带优先级继承的 futex 尝试加锁操作，非阻塞版本。

---

### FUTEX_WAIT_BITSET
```c
#define FUTEX_WAIT_BITSET 9
```
```
pub const FUTEX_WAIT_BITSET: i32 = 9;
```

[Visibility]: Internal — Linux futex 系统调用操作码

**Intent**: 带有 bitset 的 futex 等待操作。允许按位掩码选择性等待，是实现 `pthread_cond_timedwait` 的底层原语。

---

### FUTEX_PRIVATE
```c
#define FUTEX_PRIVATE 128
```
```
pub const FUTEX_PRIVATE: i32 = 128;
```

[Visibility]: Internal — Linux futex 操作修饰标志

**Intent**: 修饰标志，表示 futex 仅在进程内共享（不跨进程）。设置此标志后内核可以跳过 Futex 全局哈希表的查找，显著降低开销。musl 中所有进程内同步操作均使用 `FUTEX_PRIVATE`。

**Invariant**: `FUTEX_PRIVATE` 通过位或 (`|`) 与 futex 操作码组合使用：`FUTEX_WAIT | FUTEX_PRIVATE = 128`。

---

### FUTEX_CLOCK_REALTIME
```c
#define FUTEX_CLOCK_REALTIME 256
```
```
pub const FUTEX_CLOCK_REALTIME: i32 = 256;
```

[Visibility]: Internal — Linux futex 操作修饰标志

**Intent**: 修饰标志，表示超时使用 `CLOCK_REALTIME` 而非默认的 `CLOCK_MONOTONIC`。与 `FUTEX_WAIT_BITSET` 组合使用。

---

## 系统算法 (System Algorithm)

musl 的所有线程同步原语均建立在 futex 之上，遵循以下通用模式：

1. **用户态快速路径**: 首先通过原子操作在用户态尝试获取锁/信号量，若成功则无系统调用开销。
2. **内核态慢路径**: 若用户态竞争失败，通过 `SYS_futex` 系统调用进入内核等待。
3. **FUTEX_PRIVATE 优先**: 所有进程内同步均附加 `FUTEX_PRIVATE` 标志以减少内核开销。

## 不变量 (Invariants)

- 所有 futex 操作码（0-9）与 Linux 内核 `include/uapi/linux/futex.h` 保持严格一致。
- `FUTEX_PRIVATE` (128) 和 `FUTEX_CLOCK_REALTIME` (256) 作为位掩码标志，必须与操作码通过 `|` 组合，不可单独使用。
- futex 字必须是 32 位对齐的 `atomic int` 或等价类型。