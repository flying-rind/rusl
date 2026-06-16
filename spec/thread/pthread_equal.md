# pthread_equal.c 规约

> musl libc 的线程标识符比较函数实现。直接使用指针相等比较。

---

## 依赖图

```
pthread_equal (弱别名 -> __pthread_equal)
thrd_equal (C11, 弱别名 -> __pthread_equal)
  (无内部依赖)
```

---

## 内部静态函数规约

### 1. __pthread_equal (static)

```c
static int __pthread_equal(pthread_t a, pthread_t b);
```

[Visibility]: Internal (不导出) — static 函数，提供 `pthread_equal` 和 `thrd_equal` 的统一实现

#### Intent

比较两个 `pthread_t` 是否表示同一个线程。

#### 系统算法

```
__pthread_equal(a, b):
  1. return a == b  // 直接指针比较
```

---

## 对外导出函数规约

### 2. pthread_equal

```c
int pthread_equal(pthread_t a, pthread_t b);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)，是 `__pthread_equal` 的弱别名

#### Intent

比较两个线程标识符是否相等。`pthread_t` 是不透明类型，不可直接使用 `==` 比较（可移植性要求），必须通过此函数。

#### 前置条件

- `a` 和 `b` 为有效的 `pthread_t` 值（来自 `pthread_self()` 或 `pthread_create()` 的输出参数）

#### 后置条件

- 若 `a` 和 `b` 指向同一个线程，返回非零值（在 musl 中为 1）
- 若 `a` 和 `b` 指向不同线程，返回 0

#### 系统算法

```
pthread_equal(a, b):
  1. 等同于 __pthread_equal(a, b)
     = a == b    // musl 中 pthread_t 为指针
```

#### 注意事项

- `<pthread.h>` 中同时提供了同名的宏定义 `#define pthread_equal(x,y) ((x)==(y))`，C++ 下不可用，由编译器/环境选择使用宏或函数

### 3. thrd_equal

```c
int thrd_equal(pthread_t a, pthread_t b);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11)，是 `__pthread_equal` 的弱别名

#### Intent

C11 标准版的线程标识符比较函数。

#### 系统算法

```
thrd_equal(a, b):
  1. 等同于 __pthread_equal(a, b)
```

---

#### 不变量

无。

#### 依赖

无内部依赖。仅依赖 `<pthread.h>` / `<threads.h>` 中 `pthread_t` 类型定义。
