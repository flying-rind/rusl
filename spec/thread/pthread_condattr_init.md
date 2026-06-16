# pthread_condattr_init.c 规约

> musl libc 条件变量属性对象初始化函数。

---

## 依赖图

```
pthread_condattr_init
  (无内部依赖)
```

---

## 函数规约

### 1. pthread_condattr_init

```c
int pthread_condattr_init(pthread_condattr_t *a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

将条件变量属性对象初始化为默认值。默认值包括：时钟为 `CLOCK_REALTIME`（`0`），非进程共享。

#### 前置条件

- `a != NULL`，指向未初始化的 `pthread_condattr_t` 对象

#### 后置条件

- 始终返回 `0`（成功）
- `*a` 被零初始化：`a->__attr = 0`
- 零值对应的默认语义：
  - 时钟类型 = `CLOCK_REALTIME`（`__attr` 低 31 位为 0）
  - 非进程共享（`__attr` 最高位为 0）

#### 系统算法

```
pthread_condattr_init(a):
  1. *a = (pthread_condattr_t){0}  // 复合字面量零初始化
  2. return 0
```

#### 不变量

无。

#### 依赖

- `pthread_condattr_t` — 定义于 `<pthread.h>`，实质为 `struct { unsigned __attr; }`
