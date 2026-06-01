# memcpy — Rust 接口归约

## 原始 C 接口
```c
void *memcpy(void *restrict dest, const void *restrict src, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn memcpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void;
```

---

## 意图
将 src 的前 n 字节复制到 dest，调用者保证不重叠。对外导出 C ABI 兼容的 `memcpy` 符号。

## 前置条件
- `dest` 非空、`src` 非空
- `dest` 和 `src` 不重叠（违反则行为未定义）
- `dest` 至少可写 n 字节，`src` 至少可读 n 字节

## 后置条件
- dest[0..n-1] == src[0..n-1]
- 返回 dest

## 不变量
- 复制过程中指针变化量与 n 变化量一致

## 算法
使用对齐优化和字级别复制。Rust 中直接使用 `copy_from_slice`：

```rust
pub fn memcpy_impl(dst: &mut [u8], src: &[u8]) {
    dst.copy_from_slice(src);
}
```

若需对齐优化，可手写使用 `ptr::copy_nonoverlapping` 实现类似 C 的字级别加速路径。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::copy_nonoverlapping  // 依赖1: 非重叠内存复制原语
  core::slice::copy_from_slice    // 依赖2: 安全复制封装

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn memcpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn memcpy_impl(dst: &mut [u8], src: &[u8]);