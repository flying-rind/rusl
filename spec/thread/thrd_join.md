# thrd_join.c 规约

> musl libc 的 C11 线程等待函数实现。是 POSIX `__pthread_join` 的包装器，负责将 POSIX 的 `void *` 退出码转换为 C11 的 `int`。

---

## 依赖图

```
thrd_join
  └─> __pthread_join(t, &pthread_res)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. thrd_join

```c
int thrd_join(thrd_t t, int *res);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.4)

#### Intent

阻塞调用线程直到线程 `t` 终止，并可选地获取其退出码。是 `__pthread_join` 的包装器，将 POSIX 的 `void *` 退出码通过 `intptr_t` 转换为 `int` 存入 `*res`。

#### 前置条件

- `t` 是一个有效的、可 join 的线程标识符（未被 `thrd_detach` 分离）
- 尚未有其他线程在对 `t` 调用 `thrd_join`
- `res` 可为 `NULL`（不关心退出码）或指向有效 `int` 的指针

#### 后置条件

- 调用线程阻塞直到线程 `t` 终止
- 若 `res != NULL`：`*res = (int)(intptr_t)pthread_res`，即线程 `t` 传递给 `thrd_exit` 的值或 `func` 的返回值
- 线程 `t` 的资源被回收，不再可 join
- 始终返回 `thrd_success` (0)

#### 系统算法

```
thrd_join(t, res):
  1. __pthread_join(t, &pthread_res)   // 阻塞等待 t 终止，获取 void* 退出码
  2. if (res) *res = (int)(intptr_t)pthread_res   // 转换为 int
  3. return thrd_success
```

#### 不变量

- 每个可 join 线程恰好被 join 一次（或 detach 一次）
- 经 `intptr_t` 往返转换：`thrd_exit(v)` -> `thrd_join` -> `v`（无损）

#### 依赖

- `__pthread_join()` — POSIX 线程 join 的内部实现（见 `pthread_impl.h`）
- `intptr_t` — 足以承载指针值的整数类型（见 `<stdint.h>`）
- `thrd_t` — C11 线程标识符类型（见 `<threads.h>`）
- `thrd_success` — C11 枚举值 `0`
