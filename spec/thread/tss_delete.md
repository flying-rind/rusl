# tss_delete.c 规约

> musl libc 的 C11 线程特定存储键删除函数实现。是 POSIX `__pthread_key_delete` 的直接转发。

---

## 依赖图

```
tss_delete
  └─> __pthread_key_delete(key)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. tss_delete

```c
void tss_delete(tss_t key);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.6.2)

#### Intent

删除线程特定存储键 `key`。注意：删除键不会触发已存储值的析构函数。是 `__pthread_key_delete` 的直接转发调用。

#### 前置条件

- `key` 是通过 `tss_create` 创建的有效 TSS 键
- 调用 `tss_delete` 后不应用使用此键
- 确认没有其他线程在 `key` 上拥有待析构的非 NULL 值（否则这些值永不析构）

#### 后置条件

- TSS 键 `key` 被释放，后续 `tss_set` / `tss_get` 对该键的访问行为未定义
- 不会为任何线程触发析构函数
- 函数无返回值

#### 系统算法

```
tss_delete(key):
  1. __pthread_key_delete(key)
  2. return
```

#### 不变量

- 键的析构函数在 `tss_delete` 调用后不再被自动调用

#### 依赖

- `__pthread_key_delete()` — POSIX 线程特定数据键删除的内部实现（见 `pthread_impl.h`）
- `tss_t` — C11 TSS 键类型，`typedef unsigned`
