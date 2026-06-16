# pthread_mutex_destroy.c 规约

> musl libc pthread 互斥锁销毁。对于进程共享互斥锁，销毁前等待任何残留的 robust list 操作完成。

---

## 依赖图

```
pthread_mutex_destroy
  └── __vm_wait()  — 条件调用：仅当互斥锁为进程共享且跟踪所有权时
```

---

## 函数规约

### 1. pthread_mutex_destroy

```c
int pthread_mutex_destroy(pthread_mutex_t *mutex);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

销毁互斥锁对象。若互斥锁为进程共享且跟踪所有权（_m_type > 128 意味着至少设置了 process-shared 位），则在销毁前需等待虚拟内存区域的静默期（quiescence），确保互斥锁不再被其他进程的 robust list 引用。

#### 前置条件

- `mutex != NULL`，指向一个已初始化的 `pthread_mutex_t`
- 互斥锁应处于未加锁状态（POSIX 要求，musl 不强制检查）
- 销毁后不能再使用该互斥锁

#### 后置条件

- Case 1（总是成功）：
  - 若 `mutex->_m_type > 128`：调用 `__vm_wait()` 等待 robust list 静默
  - 返回值为 `0`

#### 系统算法

```
pthread_mutex_destroy(mutex):
  1. if mutex->_m_type > 128:
       __vm_wait()     // 等待进程共享互斥锁的 pending 状态清除
  2. return 0
```

#### 不变量

- 调用 `__vm_wait()` 后，互斥锁不再被任何 robust_list pending 槽引用

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `__vm_wait()` | `pthread_impl.h` (内部) | 等待虚拟内存区域静默期（所有 VM lock 释放） |
