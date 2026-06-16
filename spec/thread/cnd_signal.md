# cnd_signal.c 规约

> musl libc 的 C11 条件变量单信号函数实现。唤醒恰好一个在条件变量上等待的线程。

---

## 依赖图

```
cnd_signal
  └─> __private_cond_signal((pthread_cond_t *)c, 1)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. cnd_signal

```c
int cnd_signal(cnd_t *c);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.4)

#### Intent

唤醒恰好一个在条件变量 `c` 上阻塞等待的线程。若无线程等待，则无效果。是 `__private_cond_signal` 的转发包装器，传递 `1` 表示"唤醒一个"。

#### 前置条件

- `c != NULL`，指向通过 `cnd_init` 初始化的条件变量对象
- 条件变量未被销毁

#### 后置条件

- Case 1 有线程正在 `c` 上等待：恰好一个线程被唤醒，在重新获取关联互斥锁后从 `cnd_wait` 或 `cnd_timedwait` 返回
- Case 2 无线程等待：空操作
- 始终返回 `thrd_success` (0)（`__private_cond_signal` 从不失败）

#### 系统算法

```
cnd_signal(c):
  1. return __private_cond_signal((pthread_cond_t *)c, 1)
     // 参数 1 表示仅唤醒一个等待者
```

#### 不变量

- 被唤醒的线程在从等待函数返回前必须重新获取关联的互斥锁

#### 依赖

- `__private_cond_signal()` — POSIX 内部条件变量信号函数，第二个参数为正数时唤醒 n 个线程（见 `pthread_impl.h`）
- `cnd_t` — 等同于 `pthread_cond_t` 的 typedef（见 `<bits/alltypes.h>`）
- `thrd_success` — 枚举值 `0`
