# mtx_unlock.c 规约

> musl libc 的 C11 互斥锁解锁函数实现。是 POSIX `__pthread_mutex_unlock` 的直接转发，假使其不会失败。

---

## 依赖图

```
mtx_unlock
  └─> __pthread_mutex_unlock((pthread_mutex_t *)mtx)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. mtx_unlock

```c
int mtx_unlock(mtx_t *mtx);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.6)

#### Intent

解锁由调用线程持有的互斥锁。若存在等待该互斥锁的线程，则其中一个被唤醒以获取锁。是 `__pthread_mutex_unlock` 的直接转发调用。

#### 前置条件

- `mtx != NULL`，指向通过 `mtx_init` 初始化的互斥锁对象
- 调用线程必须持有该互斥锁（`mtx_lock` / `mtx_trylock` 先前成功），否则行为未定义
- 递归互斥锁需匹配 `mtx_lock` 调用次数

#### 后置条件

- 互斥锁被释放。若有等待者则其中一个被唤醒
- 返回值透传自 `__pthread_mutex_unlock`。注释指出对于 C11 语义，`pthread_mutex_unlock` 返回错误的情况皆属未定义行为（如解锁非持有的锁），故假设始终成功

#### 系统算法

```
mtx_unlock(mtx):
  1. return __pthread_mutex_unlock((pthread_mutex_t *)mtx)
     // 尾调用优化: 直接返回 POSIX 内部实现结果
     // 注: pthread_mutex_unlock 可能返回 EPERM/EINVAL, 但 C11 中这些为 UB
```

#### 不变量

- 解锁后调用线程不再持有该互斥锁

#### 依赖

- `__pthread_mutex_unlock()` — POSIX 互斥锁解锁的内部实现（见 `pthread_impl.h`）
- `mtx_t` — C11 互斥锁类型，等同于 `pthread_mutex_t` 的 typedef（见 `<bits/alltypes.h>`）
