# strncat — Rust 接口归约

## 原始 C 接口
```c
char *strncat(char *restrict d, const char *restrict s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strncat(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char;
```

---

## 意图
将 s 中最多 n 个字符追加到 d 末尾，始终追加终止 null。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- d 和 s 以 null 结尾
- d 缓冲区至少可容纳 strlen(d) + min(n, strlen(s)) + 1 字节

## 后置条件
- d 末尾追加 s 前 min(n, strlen(s)) 个字符和 '\0'
- 返回 d

## 不变量
- 最多写入 min(n, strlen(s)) + 1 字节

## 算法
找到 d 末尾，复制最多 n 个非 null 字符：

```rust
pub fn strncat_impl(d: &mut [u8], s: &core::ffi::CStr, n: usize) -> *mut u8 {
    let pos = d.iter().position(|&b| b == 0).unwrap();
    let src_bytes = s.to_bytes();
    let copy = n.min(src_bytes.len());
    d[pos..pos + copy].copy_from_slice(&src_bytes[..copy]);
    d[pos + copy] = 0;
    unsafe { d.as_mut_ptr() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes       // 依赖1: 字节切片
  core::slice::copy_from_slice    // 依赖2: 非重叠复制
  core::iter::Iterator::position  // 依赖3: 查找 null 位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strncat(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strncat_impl(d: &mut [u8], s: &core::ffi::CStr, n: usize) -> *mut u8;