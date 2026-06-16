# pthread_cond_init.c 规约

> musl libc 条件变量初始化函数。

---

## 依赖图

```
pthread_cond_init
  (无内部依赖)
```

---

## 函数规约

### 1. pthread_cond_init

```c
int pthread_cond_init(pthread_cond_t *restrict c, const pthread_condattr_t *restrict a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

初始化条件变量对象。使用默认设置或指定的属性对象（时钟类型、进程共享）配置条件变量内部字段。

#### 前置条件

- `c != NULL`，指向未初始化的 `pthread_cond_t` 对象
- `a` 可以为 `NULL`（使用默认属性），或指向有效的 `pthread_condattr_t` 对象

#### 后置条件

- 始终返回 `0`（成功）
- `*c` 被零初始化
- 若 `a != NULL`：
  - `c->_c_clock = a->__attr & 0x7fffffff`（从属性提取时钟 ID）
  - 若 `a->__attr >> 31`（进程共享标志），则 `c->_c_shared = (void *)-1`（标记为进程共享）
- 若 `a == NULL`：
  - 时钟默认为 0（`CLOCK_REALTIME`）
  - 默认非进程共享（`_c_shared = NULL`）

#### 系统算法

```
pthread_cond_init(c, a):
  1. *c = (pthread_cond_t){0}              // 零初始化全部字段
  2. if a != NULL:
  3.     c->_c_clock = a->__attr & 0x7fffffff   // 提取时钟 ID（低 31 位）
  4.     if a->__attr >> 31:                    // 最高位 = 进程共享标志
  5.         c->_c_shared = (void *)-1           // 设置进程共享标记
  6. return 0
```

#### 不变量

- 初始化后的条件变量 `_c_clock` 字段为有效时钟 ID
- `_c_shared` 为 `NULL`（非共享）或 `(void*)-1`（共享），此值用作布尔判断

#### 依赖

- `pthread_cond_t` — 定义于 `<pthread.h>`，内含 `_c_clock`, `_c_shared` 等字段
- `pthread_condattr_t` — 定义于 `<pthread.h>`，实质为 `struct { unsigned __attr; }`
