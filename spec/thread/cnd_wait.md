# cnd_wait.c 规约

> musl libc 的 C11 条件变量无条件等待函数实现。在条件变量上阻塞直至被信号唤醒。

---

## 依赖图

```
cnd_wait
  └─> cnd_timedwait(c, m, 0)  — see cnd_timedwait.c (同模块内部转发)
```

---

## 函数规约

### 1. cnd_wait

```c
int cnd_wait(cnd_t *c, mtx_t *m);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.3)

#### Intent

原子地释放互斥锁 `m` 并在条件变量 `c` 上阻塞调用线程（无限期等待），直到被 `cnd_signal` 或 `cnd_broadcast` 唤醒。被唤醒后重新获取 `m`。通过调用 `cnd_timedwait(c, m, 0)` 实现，利用 musl 内部扩展：将 `NULL` 时间戳指针传给 `cnd_timedwait` 表示无限期等待。

#### 前置条件

- `c != NULL`，指向通过 `cnd_init` 初始化的条件变量
- `m != NULL`，指向通过 `mtx_init` 初始化的互斥锁
- 调用线程必须已锁定 `m`

#### 后置条件

- 返回前互斥锁 `m` 已被调用线程重新锁定
- Case 1 被 `cnd_signal` / `cnd_broadcast` 唤醒：返回 `thrd_success` (0)
- Case 2 其他错误：返回 `thrd_error` (2)

#### 系统算法

```
cnd_wait(c, m):
  1. return cnd_timedwait(c, m, 0)
     // musl 扩展: NULL 时间戳 => 无限期等待
```

#### 不变量

- 调用线程在阻塞期间不持有互斥锁 `m`，返回时重新持有

#### 依赖

- `cnd_timedwait()` — C11 条件变量定时等待函数（见 `cnd_timedwait.c` spec），支持 `ts == NULL` 作为无限期等待的扩展语义
- `cnd_t` / `mtx_t` — C11 互斥锁/条件变量类型（见 `<bits/alltypes.h>`）
- `thrd_success` / `thrd_error` — C11 枚举值 `0` / `2`
