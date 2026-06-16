# pthread_barrier_destroy.c 规约

> musl libc 销毁屏障对象。

---

## 依赖图

```
pthread_barrier_destroy
  ├─> a_or(&b->_b_lock, INT_MIN)      (原子操作)
  ├─> __wait(&b->_b_lock, 0, v, 0)    (futex 等待)
  └─> __vm_wait()                     (Internal, SM 锁等待)
```

---

## 函数规约

### 1. pthread_barrier_destroy

```c
int pthread_barrier_destroy(pthread_barrier_t *b);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

销毁屏障对象。若屏障为进程共享（`_b_limit < 0`），且 `_b_lock` 非零（有线程正使用或等待重用屏障），则设置销毁标志并忙等全部线程退出。

#### 前置条件

- `b != NULL`，指向有效的 `pthread_barrier_t` 对象
- 无任何线程正在调用 `pthread_barrier_wait` 使用该屏障（除了当前正在等待且即将被销毁过程排空的线程）

#### 后置条件

- 始终返回 `0`
- **非进程共享屏障**（`_b_limit >= 0`）：
  - 立即返回，无任何清理操作（依赖调用者管理生命周期）
- **进程共享屏障**（`_b_limit < 0`）：
  - 若 `_b_lock != 0`（有等待者）：
    - 在 `_b_lock` 上原子 OR `INT_MIN`，标记 "销毁中"
    - 忙等直到 `_b_lock` 的低 31 位降到 0（所有线程退出屏障）
    - 调用 `__vm_wait()` 等待所有内存映射操作完成，确保共享内存安全
  - 若 `_b_lock == 0`：立即返回

#### 系统算法

```
pthread_barrier_destroy(b):
  1. if b->_b_limit < 0:                   // 进程共享屏障
  2.     if b->_b_lock != 0:               // 有线程在使用
  3.         a_or(&b->_b_lock, INT_MIN)     // 设置 INT_MIN 标志（销毁信号）
  4.         while b->_b_lock & INT_MAX:    // 等待所有线程退出
  5.             __wait(&b->_b_lock, 0, v, 0)
  6.     __vm_wait()                        // 确保 VM 操作完成
  7. return 0
```

注意：`INT_MIN` 标志位用于 `pthread_barrier_wait` 中的 `pshared_barrier_wait` 函数识别销毁状态，配合自同步销毁安全的解锁逻辑（`v == INT_MIN+1` 分支）。

#### 不变量

- 进程共享屏障销毁时必须确保无任何线程（可能在其他进程中）仍在使用该屏障
- `__vm_wait()` 调用确保所有共享内存映射的解映射操作已完成

#### 依赖

- `pthread_barrier_t` — 定义于 `<pthread.h>`，内含 `_b_limit`, `_b_lock` 字段
- `a_or()` — 原子或操作（定义于 `atomic.h`）
- `__wait()` — futex 等待（定义于 `pthread_impl.h`）
- `__vm_wait()` — 虚拟内存锁等待，确保共享内存安全（定义于 `vmlock.c`）
- `INT_MIN` — 来自 `<limits.h>`
