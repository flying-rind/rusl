# stpcpy — Rust 接口归约

## 原始 C 接口
```c
char *stpcpy(char *restrict d, const char *restrict s);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn stpcpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
将 s 指向的字符串（含终止 null）复制到 d，返回 d 中终止 null 的位置。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- `d` 至少可写 strlen(s) + 1 字节
- s 以 null 结尾

## 后置条件
- d[0..len] == s[0..len]（len == strlen(s)），包括终止 null
- 返回 &d[len]

## 不变量
- d 和 s 指针增量相等

## 算法
逐字节复制，通过字节级或字级检测 null 终止符：

```rust
pub fn stpcpy_impl(dst: &mut [u8], src: &core::ffi::CStr) -> *mut u8 {
    let bytes = src.to_bytes_with_nul();
    dst[..bytes.len()].copy_from_slice(bytes);
    unsafe { dst.as_mut_ptr().add(bytes.len() - 1) }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes_with_nul  // 依赖1: 含 null 的字节切片
  core::slice::copy_from_slice         // 依赖2: 非重叠复制

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn stpcpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn stpcpy_impl(dst: &mut [u8], src: &core::ffi::CStr) -> *mut u8;