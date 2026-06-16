# pthread_getspecific — Rust 接口归约

## 原始 C 接口
```c
static void *__pthread_getspecific(pthread_key_t k);
void *pthread_getspecific(pthread_key_t k);
void *tss_get(tss_t k);
```

`pthread_getspecific` 是 `__pthread_getspecific` 的 weak_alias；`tss_get`（C11 threads API）也是 `__pthread_getspecific` 的 weak_alias。

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn pthread_getspecific(k: pthread_key_t) -> *mut core::ffi::c_void;
extern "C" fn tss_get(k: tss_t) -> *mut core::ffi::c_void;
```

`tss_t` 与 `pthread_key_t` 同义。

---

## 意图

读取调用线程中键 `k` 关联的线程局部存储（TSD）值。直接索引 `self->tsd[k]` 返回，纯读取操作。rusl 内部实现可完全用 Safe Rust 重构：原始 C 的 `__pthread_getspecific` 是 `static` 函数（不对外导出），rusl 中可直接在 `extern "C"` 函数内部通过获取当前线程的 TSD 数组引用来实现。

## 前置条件

- `k` 是有效的 `pthread_key_t` 值（0 <= k < PTHREAD_KEYS_MAX）
- 调用线程的 `self.tsd` 已初始化（非空）

## 后置条件

- 返回 `self.tsd[k]`（可能为 `null`，表示该键未设置值）

## 不变量

- 不修改任何线程状态（纯读取操作）
- 无锁操作，依赖调用者确保 TSD 数组有效

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部实现直接通过 Rust 的安全抽象获取当前线程的 TSD 数组并索引读取。

```rust
// extern "C" 函数内部流程（safe Rust）
// 1. 通过 crate 内部的线程局部存储机制获取当前线程的 TSD 数组引用
//    例如：thread_local! { static TSD: RefCell<[*mut c_void; PTHREAD_KEYS_MAX]> = ... }
//    或通过 __pthread_self() 返回的结构体中的 tsd 字段
// 2. 返回 tsd[k]（直接索引，无锁）
```

pthread_getspecific(k):
1. 获取当前线程的 TSD 数组引用
2. 返回 `tsd[k]`（即 `Option<*mut c_void>` 对应的裸指针值）

tss_get(k):
1. 与 `pthread_getspecific` 完全相同（别名）

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供 crate 内部使用）
pub(crate) fn getspecific(k: pthread_key_t) -> *mut core::ffi::c_void;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_self()                                   // 依赖1: 获取当前线程结构体（含 tsd 数组）
  PTHREAD_KEYS_MAX                                   // 依赖2: TSD 键最大数（128）
Predefined Macros/Traits:
  (无)                                                // 纯读取操作，无额外宏依赖

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_getspecific(k: pthread_key_t) -> *mut core::ffi::c_void;
  extern "C" fn tss_get(k: tss_t) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn getspecific(k: pthread_key_t) -> *mut core::ffi::c_void;
