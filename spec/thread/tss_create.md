# tss_create.c 规约

> musl libc 的 C11 线程特定存储键创建函数实现。是 POSIX `__pthread_key_create` 的包装器，将错误信息统一归并为成功/失败。

---

## 依赖图

```
tss_create
  └─> __pthread_key_create(tss, dtor)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. tss_create

```c
int tss_create(tss_t *tss, tss_dtor_t dtor);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.6.1)

#### Intent

创建一个线程特定存储 (TSS) 键，并可选地关联析构函数 `dtor`。每个线程可通过 `tss_set` 存储与该键关联的 `void *` 值。线程退出时若 TSS 值非 NULL，则自动调用 `dtor(value)`。是 `__pthread_key_create` 的包装器，将 POSIX 多种错误码统一映射为 C11 的 `thrd_success` / `thrd_error`。

#### 前置条件

- `tss != NULL`，指向有效的 `tss_t` 内存位置
- `dtor` 可为 `NULL`（无析构）或有效函数指针 `void (*)(void *)`
- 有足够的 TSS 键可用（每个进程最多 `PTHREAD_KEYS_MAX` 个键）

#### 后置条件

- Case 1 成功（`__pthread_key_create` 返回 0）：`*tss` 存储新的 TSS 键，各线程初始关联值为 NULL，返回 `thrd_success` (0)
- Case 2 失败（键已用完等，`__pthread_key_create` 返回非零）：返回 `thrd_error` (2)

#### 系统算法

```
tss_create(tss, dtor):
  1. // POSIX 内部可能返回多种非零错误码 (EAGAIN, ENOMEM)
  2. // C11 要求统一归并为 thrd_error
  3. return __pthread_key_create(tss, dtor) ? thrd_error : thrd_success
```

#### 不变量

- 每个线程退出时，若其 TSS 值非 NULL，则执行 `dtor` 至少 `TSS_DTOR_ITERATIONS` (4) 次

#### 依赖

- `__pthread_key_create()` — POSIX 线程特定数据键创建的内部实现（见 `pthread_impl.h`）
- `tss_t` — C11 TSS 键类型，`typedef unsigned`
- `tss_dtor_t` — `void (*)(void *)` 函数指针类型
- `thrd_success` / `thrd_error` — C11 枚举值 `0` / `2`
