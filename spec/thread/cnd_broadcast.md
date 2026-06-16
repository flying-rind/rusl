# cnd_broadcast.c 规约

> musl libc 的 C11 条件变量广播函数实现。唤醒所有在条件变量上等待的线程。

---

## 依赖图

```
cnd_broadcast
  └─> __private_cond_signal((pthread_cond_t *)c, -1)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. cnd_broadcast

```c
int cnd_broadcast(cnd_t *c);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.5)

#### Intent

唤醒所有在条件变量 `c` 上阻塞等待的线程。若无线程等待，则无效果。是 `__private_cond_signal` 的转发包装器，传递 `-1` 表示"广播全部"。

#### 前置条件

- `c != NULL`，指向通过 `cnd_init` 初始化的条件变量对象
- 条件变量未被销毁
- 调用者应当在持有与条件变量关联的互斥锁时调用（非必须但推荐）

#### 后置条件

- Case 1 有线程正在 `c` 上等待：所有等待线程被唤醒，每个线程在重新获取关联互斥锁后从 `cnd_wait` 或 `cnd_timedwait` 返回
- Case 2 无线程等待：空操作
- 始终返回 `thrd_success` (0)（`__private_cond_signal` 从不失败）

#### 系统算法

```
cnd_broadcast(c):
  1. return __private_cond_signal((pthread_cond_t *)c, -1)
     // 参数 -1 表示唤醒所有等待者（而非仅唤醒 1 个）
```

#### 不变量

- 被唤醒的线程在从等待函数返回前必须重新获取关联的互斥锁

#### 依赖

- `__private_cond_signal()` — POSIX 内部条件变量信号函数，第二个参数为正数时唤醒 n 个线程，为 -1 时唤醒全部（见 `pthread_impl.h`）
- `cnd_t` — 等同于 `pthread_cond_t` 的 typedef（见 `<bits/alltypes.h>`）
- `thrd_success` — 枚举值 `0`
