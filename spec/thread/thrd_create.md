# thrd_create.c 规约

> musl libc 的 C11 线程创建函数实现。是 POSIX `__pthread_create` 的包装器，负责将 POSIX 返回值映射为 C11 返回值并使用 C11 线程属性。

---

## 依赖图

```
thrd_create
  └─> __pthread_create(thr, __ATTRP_C11_THREAD, (void *(*)(void *))func, arg)
        — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. thrd_create

```c
int thrd_create(thrd_t *thr, thrd_start_t func, void *arg);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.1)

#### Intent

创建一个新的执行线程，执行 `func(arg)`，并将线程标识符存入 `*thr`。是 `__pthread_create` 的包装器，传递特殊属性 `__ATTRP_C11_THREAD` 以指示这是 C11 线程（区别于 POSIX 线程），并负责 C 类型转换与返回值映射。

#### 前置条件

- `thr != NULL`，指向有效的 `thrd_t` 内存位置
- `func != NULL`，签名 `int (*)(void *)` 的有效函数指针
- `arg` 可为任意指针值
- 系统资源足够创建新线程

#### 后置条件

- Case 1 成功（`ret == 0`）：`*thr` 存储新线程标识符，线程开始执行 `func(arg)`，返回 `thrd_success` (0)
- Case 2 内存不足（`ret == EAGAIN`）：线程无法创建，返回 `thrd_nomem` (3)
- Case 3 其他错误：返回 `thrd_error` (2)

#### 系统算法

```
thrd_create(thr, func, arg):
  1. // 转换函数类型: C11 thrd_start_t (int (*)(void*)) -> POSIX (void *(*)(void*))
  2. ret = __pthread_create(thr, __ATTRP_C11_THREAD, (void *(*)(void *))func, arg)
  3. switch (ret):
       case 0:      return thrd_success  // 成功
       case EAGAIN: return thrd_nomem    // 系统资源不足
       default:     return thrd_error    // 其他错误
```

#### 不变量

- 新线程与创建线程并发执行，共享进程地址空间
- C11 线程由内部属性 `__ATTRP_C11_THREAD` 标记，用于 exit/join 语义区分

#### 依赖

- `__pthread_create()` — POSIX 线程创建的内部实现（见 `pthread_impl.h`）
- `__ATTRP_C11_THREAD` — `((void*)(uintptr_t)-1)`，C11 线程属性标记（见 `pthread_impl.h`）
- `thrd_t` — C11 线程标识符类型：C 中为 `struct __pthread *`，C++ 中为 `unsigned long`（见 `<threads.h>`）
- `thrd_start_t` — `int (*)(void *)` 函数指针类型
- `thrd_success` / `thrd_nomem` / `thrd_error` — C11 枚举值 `0` / `3` / `2`
- `EAGAIN` — errno 值，资源暂时不可用
