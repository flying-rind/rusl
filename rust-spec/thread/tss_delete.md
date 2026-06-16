# tss_delete — Rust 接口归约

## 原始 C 接口
```c
void tss_delete(tss_t key);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.6.2)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// tss_t = c_uint，值传递（非指针）
extern "C" fn tss_delete(key: core::ffi::c_uint);
```

---

## 意图
删除线程特定存储键 `key`。注意：删除键不会触发已存储值的析构函数。是 `__pthread_key_delete` 的直接转发调用。

## 前置条件
- `key` 是通过 `tss_create` 创建的有效 TSS 键
- 调用 `tss_delete` 后不应用使用此键
- 确认没有其他线程在 `key` 上拥有待析构的非 NULL 值（否则这些值永不析构）

## 后置条件
- TSS 键 `key` 被释放，后续 `tss_set` / `tss_get` 对该键的访问行为未定义
- 不会为任何线程触发析构函数
- 函数无返回值

## 不变量
- 键的析构函数在 `tss_delete` 调用后不再被自动调用

## 算法
```rust
extern "C" fn tss_delete(key: c_uint) {
    unsafe {
        __pthread_key_delete(key);
    }
}
```

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 POSIX TSD 键删除
pub(crate) unsafe fn __pthread_key_delete(key: c_uint);
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_key_delete(key: c_uint)    // 依赖1: musl 内部 TSD 键删除实现
Predefined Macros/Types:
  tss_t (= c_uint)                     // C11 TSS 键类型

[GUARANTEE]
Exported Interface:
  extern "C" fn tss_delete(key: core::ffi::c_uint);
                                        // 本模块保证对外提供与 C ABI 兼容的 tss_delete 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_key_delete(key: c_uint);
                                        // 内部 TSD 键删除，模块间共享
