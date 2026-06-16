# pthread_mutex_init.c 规约

> musl libc pthread 互斥锁初始化。将互斥锁对象置零并根据可选的属性对象设置类型标志。

---

## 依赖图

```
pthread_mutex_init
  └── (无函数调用 — 仅结构体赋值)
```

---

## 互斥锁类型布局

musl 中 `pthread_mutex_t` 定义为一组 union 的别名，通过以下宏访问：

```c
// 定义于 pthread_impl.h，通过 <pthread.h> 引入
// 64-bit: __i[10], __vi[10], __p[5]
// 32-bit: __i[6],  __vi[6],  __p[6]

#define _m_type    __u.__i[0]    // 类型/属性位掩码
#define _m_lock    __u.__vi[1]   // 锁状态 (owner tid / 标志位)
#define _m_waiters __u.__vi[2]   // 等待者计数/标志
#define _m_prev    __u.__p[3]    // robust list 前驱指针
#define _m_next    __u.__p[4]    // robust list 后继指针
#define _m_count   __u.__i[5]    // 递归计数 (RECURSIVE 类型)
```

PTHREAD_MUTEX_INITIALIZER `{{{0}}}` 将所有字段初始化为零。

---

## 函数规约

### 1. pthread_mutex_init

```c
int pthread_mutex_init(pthread_mutex_t *restrict m, const pthread_mutexattr_t *restrict a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

根据可选的属性对象初始化互斥锁。若属性对象为 NULL，则使用默认属性（NORMAL 类型、非递归、非健壮、进程私有）。

#### 前置条件

- `m != NULL`，指向调用者分配的 `pthread_mutex_t` 内存
- 若 `a != NULL`，则 `a` 指向一个已初始化的 `pthread_mutexattr_t`

#### 后置条件

- Case 1（总是成功）：
  - `m` 的所有 union 字段被零初始化
  - 若 `a != NULL`：`m->_m_type = a->__attr`
  - 若 `a == NULL`：`m->_m_type = 0`（默认属性）
  - 返回值为 `0`

#### 系统算法

```
pthread_mutex_init(m, a):
  1. *m = (pthread_mutex_t){0}     // 零初始化所有字段
  2. if a != NULL: m->_m_type = a->__attr
  3. return 0
```

#### 不变量

- 初始化后的互斥锁处于未加锁状态（`_m_lock == 0`）
- `_m_count == 0`（递归计数归零）
- `_m_waiters == 0`（无等待者）
- `_m_prev == NULL` / `_m_next == NULL`（未加入 robust list）

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `pthread_mutex_t` | `<pthread.h>` | 互斥锁类型 |
| `pthread_mutexattr_t` | `<pthread.h>` | 互斥锁属性类型 |
