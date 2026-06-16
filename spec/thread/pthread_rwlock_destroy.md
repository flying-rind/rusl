# pthread_rwlock_destroy.c 规约

> musl libc 读写锁销毁函数。销毁一个 `pthread_rwlock_t` 对象。

---

## 依赖图

```
pthread_rwlock_destroy (Public API)
  (无内部函数依赖，直接返回 0)
```

---

## 函数规约

### 1. pthread_rwlock_destroy

```c
int pthread_rwlock_destroy(pthread_rwlock_t *rw);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX）

#### Intent

销毁读写锁对象 `rw`，释放其占用的实现定义资源。musl 实现中读写锁不持有内核资源或动态分配的内存，因此仅返回 0。

#### 前置条件

- `rw` 指向一个已初始化的 `pthread_rwlock_t` 对象
- 调用时没有线程持有或等待该读写锁（未定义行为否则）
- 销毁后不应再次使用 `rw`，除非重新初始化

#### 后置条件

- Case 1 成功：返回 0
- 无错误分支（musl 实现总是返回 0）

#### 不变量

无。

#### 依赖

无内部依赖。仅需 `<pthread.h>` 声明的类型定义。
