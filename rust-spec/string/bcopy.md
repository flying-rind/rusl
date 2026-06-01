# bcopy — Rust 接口归约

## 原始 C 接口
```c
void bcopy(const void *s1, void *s2, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn bcopy(s1: *const core::ffi::c_void, s2: *mut core::ffi::c_void, n: usize);
```

---

## 意图
将 s1 的前 n 字节复制到 s2，支持源与目标重叠。对外导出 C ABI 兼容的 `bcopy` 符号。

## 前置条件
- `s1` 非空、`s2` 非空
- 当 `n > 0` 时，`s1` 至少可读 n 字节，`s2` 至少可写 n 字节

## 后置条件
- s2[0..n-1] == s1[0..n-1]
- 支持重叠区域安全复制

## 不变量
- 无全局或静态状态被修改

## 算法
内部委托给 `memmove`：

```rust
pub fn bcopy_impl(src: &[u8], dst: &mut [u8]) {
    dst.copy_from_slice(src);
}
// 注意: copy_from_slice 不允许重叠，内部应使用 ptr::copy
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::copy           // 依赖1: 重叠安全的内存复制
  core::slice::copy_from_slice  // 依赖2: 非重叠 slice 复制（备用）

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn bcopy(s1: *const core::ffi::c_void, s2: *mut core::ffi::c_void, n: usize);
Internal Interface:
  pub(crate) fn bcopy_impl(src: &[u8], dst: &mut [u8]);