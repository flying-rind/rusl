# wcswcs — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcswcs(const wchar_t *haystack, const wchar_t *needle);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcswcs(haystack: *const u32, needle: *const u32) -> *mut u32;
```

---

## 意图
在宽字符串 haystack 中查找子串 needle 首次出现的位置。wcsstr 的 BSD 别名。

## 前置条件
- `haystack` 非空、`needle` 非空
- haystack 和 needle 以 L'\0' 结尾

## 后置条件
- 与 wcsstr(haystack, needle) 完全相同
- 字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 wcsstr：

```rust
pub fn wcswcs_impl(haystack: &[u32], needle: &[u32]) -> Option<*const u32> {
    wcsstr_impl(haystack, needle)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  wcsstr_impl  // 依赖1: 宽字符串子串查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcswcs(haystack: *const u32, needle: *const u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcswcs_impl(haystack: &[u32], needle: &[u32]) -> Option<*const u32>;