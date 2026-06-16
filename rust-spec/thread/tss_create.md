# tss_create — Rust 接口归约

## 原始 C 接口
```c
int tss_create(tss_t *tss, tss_dtor_t dtor);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.6.1)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// tss_t = c_uint，用作 TSD 数组索引
// tss_dtor_t = Option<extern "C" fn(*mut c_void)>
extern "C" fn tss_create(
    tss: *mut core::ffi::c_uint,
    dtor: Option<extern "C" fn(*mut core::ffi::c_void)>,
) -> core::ffi::c_int;
```

---

## 意图
创建一个线程特定存储 (TSS) 键，并可选地关联析构函数 `dtor`。每个线程可通过 `tss_set` 存储与该键关联的 `void *` 值。线程退出时若 TSS 值非 NULL，则自动调用 `dtor(value)`。是 `__pthread_key_create` 的包装器，将 POSIX 多种错误码统一映射为 C11 的 `thrd_success` / `thrd_error`。

## 前置条件
- `tss` 为非空指针（`!tss.is_null()`），指向有效的 `tss_t`（`c_uint`）内存位置
- `dtor` 可为 `None`（无析构）或有效函数指针 `extern "C" fn(*mut c_void)`
- 有足够的 TSS 键可用（每个进程最多 `PTHREAD_KEYS_MAX` = 128 个键）

## 后置条件
- Case 1 成功（`__pthread_key_create` 返回 0）：`*tss` 存储新的 TSS 键，各线程初始关联值为 null，返回 `thrd_success` (0)
- Case 2 失败（键已用完等，返回非零）：返回 `thrd_error` (2)

## 不变量
- 每个线程退出时，若其 TSS 值非 NULL，则执行 `dtor` 至少 `TSS_DTOR_ITERATIONS` (4) 次

## 算法
```rust
extern "C" fn tss_create(
    tss: *mut c_uint,
    dtor: Option<extern "C" fn(*mut c_void)>,
) -> c_int {
    // POSIX 内部可能返回多种非零错误码 (EAGAIN, ENOMEM)
    // C11 要求统一归并为 thrd_error
    let ret = unsafe { __pthread_key_create(tss, dtor) };
    if ret != 0 { thrd_error } else { thrd_success }
}
```

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 POSIX TSD 键创建
pub(crate) unsafe fn __pthread_key_create(
    key: *mut c_uint,
    dtor: Option<extern "C" fn(*mut c_void)>,
) -> c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_key_create(key: *mut c_uint, dtor: Option<extern "C" fn(*mut c_void)>) -> c_int
                                      // 依赖1: musl 内部 TSD 键创建实现
Predefined Macros/Types:
  tss_t (= c_uint)                     // C11 TSS 键类型
  tss_dtor_t = Option<extern "C" fn(*mut c_void)>
  thrd_success (0) / thrd_error (2)    // C11 返回值枚举

[GUARANTEE]
Exported Interface:
  extern "C" fn tss_create(
      tss: *mut core::ffi::c_uint,
      dtor: Option<extern "C" fn(*mut core::ffi::c_void)>,
  ) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 tss_create 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_key_create(
      key: *mut c_uint,
      dtor: Option<extern "C" fn(*mut c_void)>,
  ) -> c_int;
                                        // 内部 TSD 键创建，模块间共享
