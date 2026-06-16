# pthread_mutex_consistent.c 规约

> musl libc pthread 健壮互斥锁一致性标记。在检测到互斥锁处于 EOWNERDEAD 状态后，调用者修复受保护数据后调用此函数将互斥锁标记为一致状态。

---

## 依赖图

```
pthread_mutex_consistent
  ├── __pthread_self()        — 获取当前线程 tid（所有权校验）
  └── a_and(&m->_m_lock, ~0x40000000)  — 原子清除 dead 标记
```

---

## 互斥锁位域布局（回顾）

`_m_lock` 字段：

| 位 | 掩码 | 含义 |
|----|------|------|
| 0–29 | `0x3fffffff` | 持有者 tid |
| 30 | `0x40000000` | EOWNERDEAD 标记 |
| 31 | `0x80000000` | 等待者标记 |

`_m_type` 字段位 2 (`& 4`) 为 `PTHREAD_MUTEX_ROBUST` 标志。

---

## 函数规约

### 1. pthread_mutex_consistent

```c
int pthread_mutex_consistent(pthread_mutex_t *m);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

当健壮互斥锁被获取但检测到前一持有者已终止（`EOWNERDEAD`），调用者在修复受保护的共享数据后，调用此函数将互斥锁标记为一致状态。调用者必须是当前互斥锁的持有者。

#### 前置条件

- `m != NULL`，指向一个已初始化的 `pthread_mutex_t`
- 互斥锁必须是 ROBUST 类型（`_m_type & 4` 非零）
- 互斥锁处于 EOWNERDEAD 状态（`_m_lock` 位 30 已设置）
- 互斥锁当前被某线程持有（`own = _m_lock & 0x3fffffff` 非零）
- 调用者必须已通过 `pthread_mutex_lock` 获取了此互斥锁

#### 后置条件

- Case 1 成功 — 互斥锁为 ROBUST、处于 EOWNERDEAD、被当前线程持有：
  - `_m_lock` 的位 30（`0x40000000`）被原子清除
  - 互斥锁恢复为一致状态，可作为普通锁继续使用
  - 返回值为 `0`
- Case 2 互斥锁非 ROBUST（`!(type & 4)`）、或无持有者（`own == 0`）、或未处于 EOWNERDEAD（`!(old & 0x40000000)`）：
  - 互斥锁状态不变
  - 返回值为 `EINVAL`
- Case 3 调用者非当前持有者（`own != __pthread_self()->tid`）：
  - 互斥锁状态不变
  - 返回值为 `EPERM`

#### 系统算法

```
pthread_mutex_consistent(m):
  1. old = m->_m_lock
     own = old & 0x3fffffff
  2. // 校验：必须是 ROBUST、有持有者、处于 EOWNERDEAD 状态
     if !(m->_m_type & 4) || !own || !(old & 0x40000000):
       return EINVAL
  3. // 校验：调用者必须是持有者
     if own != __pthread_self()->tid:
       return EPERM
  4. // 原子清除 EOWNERDEAD 标记
     a_and(&m->_m_lock, ~0x40000000)
  5. return 0
```

#### 不变量

- 仅清除 `_m_lock` 的 EOWNERDEAD 标记位，不影响所有者 tid 和 waiters 标志
- 操作是原子的（`a_and`），与并发的等待/锁操作正确同步

#### 使用场景

典型的 robust mutex 恢复流程为：

```
1. pthread_mutex_lock(&m) → 返回 EOWNERDEAD
2. (修复被保护数据)
3. pthread_mutex_consistent(&m) → 返回 0，标记一致性
4. 正常使用，最终 pthread_mutex_unlock(&m)
```

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `__pthread_self()` | `pthread_impl.h` (内部) | 获取当前线程控制块 |
| `a_and(p, v)` | `atomic.h` (内部) | 原子按位与（清除 EOWNERDEAD） |
