# strcpy — Rust 接口归约

## 原始 C 接口
```c
char *strcpy(char *restrict dest, const char *restrict src);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strcpy(dest: *mut core::ffi::c_char, src: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
将 src 字符串（含终止 null）复制到 dest 缓冲区。

## 前置条件
- `dest` 非空、`src` 非空
- `dest` 和 `src` 不重叠
- `dest` 至少可写 strlen(src) + 1 字节
- src 以 null 结尾

## 后置条件
- dest[0..len] == src[0..len]（len == strlen(src)），含终止 null
- 返回 dest

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 stpcpy 实现：

```rust
pub fn strcpy_impl(dest: &mut [u8], src: &core::ffi::CStr) -> *mut u8 {
    let bytes = src.to_bytes_with_nul();
    dest[..bytes.len()].copy_from_slice(bytes);
    unsafe { dest.as_mut_ptr() }
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
  extern "C" fn strcpy(dest: *mut core::ffi::c_char, src: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strcpy_impl(dest: &mut [u8], src: &core::ffi::CStr) -> *mut u8;