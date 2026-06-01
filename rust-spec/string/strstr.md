# strstr — Rust 接口归约

## 原始 C 接口
```c
char *strstr(const char *h, const char *n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strstr(h: *const core::ffi::c_char, n: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
在字符串 h（haystack）中查找子串 n（needle）首次出现的位置。

## 前置条件
- `h` 非空、`n` 非空
- h 和 n 以 null 结尾

## 后置条件
- 若 n 为空串，返回 h
- 若找到 n，返回指向 h 中首次匹配位置的指针
- 若未找到，返回 null
- 字符串内容不变

## 不变量
- Two-Way 算法中 shift 表、byteset 在搜索期间不变

## 算法
使用 Two-Way 字符串匹配算法：

```rust
pub fn strstr_impl(haystack: &[u8], needle: &[u8]) -> Option<*const u8> {
    if needle.is_empty() { return Some(haystack.as_ptr()); }
    haystack.windows(needle.len()).position(|w| w == needle)
        .map(|i| unsafe { haystack.as_ptr().add(i) })
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u8]>::windows   // 依赖1: 滑动窗口
  core::iter::Iterator::position      // 依赖2: 位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strstr(h: *const core::ffi::c_char, n: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strstr_impl(haystack: &[u8], needle: &[u8]) -> Option<*const u8>;