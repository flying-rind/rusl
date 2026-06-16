# pthread_attr_destroy.c 规约

> musl libc 线程属性对象销毁函数。musl 中 `pthread_attr_t` 不管理任何动态分配资源，因此销毁操作为空操作。

---

## 依赖图

```
pthread_attr_destroy
  (无依赖)
```

---

## 函数规约

### 1. pthread_attr_destroy

```c
int pthread_attr_destroy(pthread_attr_t *a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

销毁线程属性对象。musl 实现中 `pthread_attr_t` 是栈分配的结构体，不包含动态分配的内存或系统资源，因此该函数仅返回成功，不执行任何实际操作。

#### 前置条件

- `a != NULL`，指向一个有效的 `pthread_attr_t` 对象

#### 后置条件

- Case 1 始终：返回 `0`
- `a` 的内容不发生变化（musl 不执行清零或其他清理）

#### 不变量

无。

#### 依赖

- `pthread_impl.h` — 内部头文件，定义 `pthread_attr_t` 的成员访问宏
