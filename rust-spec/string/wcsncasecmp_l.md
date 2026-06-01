# wcsncasecmp_l — Rust 接口归约

## 原始 C 接口
```c
int wcsncasecmp_l(const wchar_t *l, const wchar_t *r, size_t n, locale_t locale);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsncasecmp_l(l: *const u32, r: *const u32, n: usize, locale: *mut core::ffi::c_void) -> core::ffi::c_int;
```

---

## 意图
在指定 locale 下忽略大小写比较两个宽字符串的前 n 个宽字符。musl 实现忽略 locale 参数。

## 前置条件
- `l` 非空、`r` 非空
- l 和 r 以 L'\0' 结尾

## 后置条件
- 与 wcsncasecmp(l, r, n) 返回相同值
- 字符串内容不变

## 不变量
- locale 参数被忽略

## 算法
直接委托给 wcsncasecmp：

```rust
pub fn wcsncasecmp_l_impl(l: &[u32], r: &[u32], n: usize, _locale: *mut core::ffi::c_void) -> core::ffi::c_int {
    wcsncasecmp_impl(l, r, n)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  wcsncasecmp_impl  // 依赖1: 忽略大小写宽字符串比较

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcsncasecmp_l(l: *const u32, r: *const u32, n: usize, locale: *mut core::ffi::c_void) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn wcsncasecmp_l_impl(l: &[u32], r: &[u32], n: usize, locale: *mut core::ffi::c_void) -> core::ffi::c_int;