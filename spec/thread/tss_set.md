# tss_set.c 规约

> musl libc 的 C11 线程特定存储写函数实现。直接访问当前线程的 TSD 数组，对非 COW 系统进行优化以避免不必要的写时复制。

---

## 依赖图

```
tss_set
  ├─> __pthread_self()                          — see pthread_impl.h (获取当前线程结构)
  ├─> struct pthread->tsd[k]                    — see pthread_impl.h (TSD 数组访问)
  ├─> struct pthread->tsd_used                  — see pthread_impl.h (TSD 使用标记)
  └─> thrd_success                              — see <threads.h> (C11 枚举)
```

---

## 函数规约

### 1. tss_set

```c
int tss_set(tss_t k, void *x);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.6.4)

#### Intent

在线程特定存储中为当前线程设置键 `k` 关联的值为 `x`。直接访问 `struct pthread` 内部的 `tsd[]` 数组实现高性能存取。包含写时复制 (COW) 优化：若新值与旧值相同则跳过写入，避免在 fork 后触发不必要的内存复制。

#### 前置条件

- `k` 是通过 `tss_create` 创建的有效 TSS 键
- 在可通过 `__pthread_self()` 获取内部线程结构的上下文中调用（非信号处理器上下文）

#### 后置条件

- `self->tsd[k] = x`（仅当旧值不等于 `x` 时执行写入）
- 若执行了写入：`self->tsd_used = 1`，标记该线程使用了 TSD（触发线程退出时的析构函数扫描）
- 始终返回 `thrd_success` (0)

#### 系统算法

```
tss_set(k, x):
  1. self = __pthread_self()              // 获取当前线程的 struct pthread *
  2. if (self->tsd[k] != x):              // COW 优化: 仅当值改变时才写入
       self->tsd[k] = x                   // 设置新值
       self->tsd_used = 1                 // 标记: 此线程有 TSD 需要析构
  3. return thrd_success
```

#### 不变量

- `self->tsd_used == 1` 当且仅当至少一个 TSS 键被设为了非 NULL 值
- COW 优化依赖于 fork 子进程只读复制父进程内存页的机制

#### 依赖

- `__pthread_self()` — 返回当前线程的 `struct pthread *`（见 `pthread_impl.h`）
- `struct pthread` — musl 线程控制块，包含 `void **tsd`（TSD 数组指针）和 `unsigned char tsd_used:1` 位域（见 `pthread_impl.h`）
- `tss_t` — C11 TSS 键类型，`typedef unsigned`（用作 TSD 数组索引）
- `thrd_success` — C11 枚举值 `0`
