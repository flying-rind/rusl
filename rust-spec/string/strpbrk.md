# strpbrk — Rust 接口归约

## 原始 C 接口
```c
char *strpbrk(const char *s, const char *b);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strpbrk(s: *const core::ffi::c_char, b: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
在字符串 s 中查找 b 中任意字符首次出现的位置。

## 前置条件
- `s` 非空、`b` 非空
- s 和 b 以 null 结尾

## 后置条件
- 若找到 b 中的任意字符，返回指向该位置的指针
- 若未找到，返回 null
- s 和 b 内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
使用 strcspn 计算位置：

```rust
pub fn strpbrk_impl(s: &core::ffi::CStr, accept: &core::ffi::CStr) -> Option<*const u8> {
    let pos = strcspn_impl(s, accept);
    let bytes = s.to_bytes_with_nul();
    if pos < bytes.len() - 1 { Some(unsafe { s.as_ptr().add(pos) as *const u8 }) } else { None }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  strcspn_impl                    // 依赖1: 计算不含 accept 字符的前缀长度
  core::ffi::CStr::to_bytes_with_nul  // 依赖2: 含 null 的字节切片

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strpbrk(s: *const core::ffi::c_char, b: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strpbrk_impl(s: &core::ffi::CStr, accept: &core::ffi::CStr) -> Option<*const u8>;