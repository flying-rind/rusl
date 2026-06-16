# pthread_mutexattr_destroy.c 规约

> musl libc pthread 互斥锁属性对象销毁。在 musl 中为无操作（属性对象不含动态分配资源）。

---

## 依赖图

```
pthread_mutexattr_destroy
  └── (无依赖 — 无操作)
```

---

## 函数规约

### 1. pthread_mutexattr_destroy

```c
int pthread_mutexattr_destroy(pthread_mutexattr_t *a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

销毁互斥锁属性对象 `a`。musl 中属性对象不含动态分配内存，因此无需释放任何资源，该函数仅返回成功。

#### 前置条件

- `a != NULL`，指向一个已初始化的 `pthread_mutexattr_t`（通过 `pthread_mutexattr_init` 或等价方式）

#### 后置条件

- Case 1（总是成功）：
  - 返回值为 `0`
  - `a` 对象的内容不再有效（语义上已销毁）

#### 系统算法

```
pthread_mutexattr_destroy(a):
  1. return 0  // musl 中无资源需要释放
```

#### 不变量

无。

#### 依赖

无。
