# wcsstr — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcsstr(const wchar_t *restrict h, const wchar_t *restrict n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsstr(h: *const u32, n: *const u32) -> *mut u32;
```

---

## 意图
在宽字符串 h（haystack）中查找子串 n（needle）首次出现的位置。

## 前置条件
- `h` 非空、`n` 非空
- h 和 n 以 L'\0' 结尾

## 后置条件
- 若 n 为空串，返回 h
- 若找到 n，返回 h 中首次匹配位置的指针
- 若未找到，返回 null
- 字符串内容不变

## 不变量
- Two-Way 算法从不回溯超过已检查范围

## 算法
使用 Two-Way 宽字符串匹配算法：

```rust
pub fn wcsstr_impl(haystack: &[u32], needle: &[u32]) -> Option<*const u32> {
    if needle.is_empty() || needle[0] == 0 { return Some(haystack.as_ptr()); }
    let nlen = needle.iter().position(|&c| c == 0).unwrap_or(needle.len());
    haystack.windows(nlen).position(|w| w == &needle[..nlen])
        .map(|i| unsafe { haystack.as_ptr().add(i) })
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u32]>::windows  // 依赖1: 滑动窗口
  core::iter::Iterator::position      // 依赖2: 位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcsstr(h: *const u32, n: *const u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcsstr_impl(haystack: &[u32], needle: &[u32]) -> Option<*const u32>;