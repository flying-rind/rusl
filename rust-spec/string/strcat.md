# strcat — Rust 接口归约

## 原始 C 接口
```c
char *strcat(char *restrict dest, const char *restrict src);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strcat(dest: *mut core::ffi::c_char, src: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
将 src 字符串追加到 dest 字符串末尾（覆盖 dest 的终止 null），包括 src 的终止 null。

## 前置条件
- `dest` 非空、`src` 非空
- `dest` 和 `src` 不重叠
- dest 和 src 以 null 结尾
- dest 缓冲区至少可容纳 strlen(dest) + strlen(src) + 1 字节

## 后置条件
- dest[dest_len..dest_len+src_len] == src[0..src_len]
- dest[dest_len+src_len] == '\0'
- 返回 dest

## 不变量
- 无全局或静态状态被修改

## 算法
找到 dest 末尾，然后复制 src：

```rust
pub fn strcat_impl(dest: &mut [u8], src: &core::ffi::CStr) -> *mut u8 {
    let dest_len = dest.iter().position(|&b| b == 0).unwrap();
    let src_bytes = src.to_bytes_with_nul();
    dest[dest_len..dest_len + src_bytes.len()].copy_from_slice(src_bytes);
    unsafe { dest.as_mut_ptr() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes_with_nul  // 依赖1: 含 null 的字节切片
  core::slice::copy_from_slice         // 依赖2: 非重叠复制
  core::iter::Iterator::position       // 依赖3: 查找 null 位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strcat(dest: *mut core::ffi::c_char, src: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strcat_impl(dest: &mut [u8], src: &core::ffi::CStr) -> *mut u8;