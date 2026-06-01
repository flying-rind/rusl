# strcasestr — Rust 接口归约

## 原始 C 接口
```c
char *strcasestr(const char *h, const char *n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strcasestr(h: *const core::ffi::c_char, n: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
在字符串 h（haystack）中忽略大小写查找子串 n（needle）首次出现的位置。

## 前置条件
- `h` 非空、`n` 非空
- h 和 n 以 null 结尾

## 后置条件
- 若 strlen(n) == 0，返回 h
- 若找到匹配子串，返回指向 h 中首次匹配位置的指针
- 若未找到，返回 null
- 字符串内容不变

## 不变量
- 外层遍历 h 每个位置，内层通过 strncasecmp 比较

## 算法
朴素 O(|h|*|n|) 算法：

```rust
pub fn strcasestr_impl(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() { return Some(0); }
    haystack.windows(needle.len()).position(|w| {
        w.iter().zip(needle.iter()).all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
    })
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u8]>::windows   // 依赖1: 滑动窗口
  u8::to_ascii_lowercase               // 依赖2: ASCII 小写转换
  core::iter::Iterator::position       // 依赖3: 位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strcasestr(h: *const core::ffi::c_char, n: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strcasestr_impl(haystack: &[u8], needle: &[u8]) -> Option<usize>;