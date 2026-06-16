# pthread_getspecific.c 规约

> musl libc 线程局部存储（TSD）值读取函数。通过弱别名导出为 `pthread_getspecific` 和 `tss_get`（C11 threads）。

---

## 依赖图

```
pthread_getspecific (weak_alias)
  └─> __pthread_getspecific
        └─> __pthread_self()  (see pthread_self.c spec)

tss_get (weak_alias)
  └─> __pthread_getspecific
        └─> __pthread_self()  (see pthread_self.c spec)
```

---

## 函数规约

### 1. `__pthread_getspecific` (static)

```c
static void *__pthread_getspecific(pthread_key_t k);
```

[Visibility]: Internal (不导出) — 文件作用域 static 函数，是 `pthread_getspecific` / `tss_get` 的真实实现

#### Intent

读取调用线程中键 `k` 关联的线程局部存储值。直接索引 `self->tsd[k]` 返回。

#### 前置条件

- `k` 是有效的 `pthread_key_t` 值（0 <= k < PTHREAD_KEYS_MAX）
- 调用线程的 `self->tsd` 已初始化（非 NULL）

#### 后置条件

- 返回 `self->tsd[k]`（可能为 NULL，表示该键未设置值）

#### 不变量

- 不修改任何线程状态（纯读取操作）
- 无锁操作，依赖调用者确保 TSD 数组有效

#### 依赖

- `__pthread_self()` — 获取调用线程的 `pthread` 结构体指针

---

### 2. `pthread_getspecific`

```c
void *pthread_getspecific(pthread_key_t k);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

`weak_alias(__pthread_getspecific, pthread_getspecific)`

规约同 `__pthread_getspecific`。

---

### 3. `tss_get`

```c
void *tss_get(tss_t k);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出（C11 threads API）

`weak_alias(__pthread_getspecific, tss_get)`

规约同 `__pthread_getspecific`。`tss_t` 与 `pthread_key_t` 同义。
