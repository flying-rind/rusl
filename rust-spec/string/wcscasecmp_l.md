# wcscasecmp_l — Rust 接口归约

## 原始 C 接口
```c
int wcscasecmp_l(const wchar_t *l, const wchar_t *r, locale_t locale);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcscasecmp_l(l: *const u32, r: *const u32, locale: *mut core::ffi::c_void) -> core::ffi::c_int;
```

---

## 意图
在指定 locale 下忽略大小写比较两个宽字符串。musl 实现中 locale 参数被忽略。

## 前置条件
- `l` 非空、`r` 非空
- l 和 r 以 L'\0' 结尾

## 后置条件
- 与 wcscasecmp(l, r) 返回相同值
- 字符串内容不变

## 不变量
- locale 参数被忽略

## 算法
直接委托给 wcscasecmp：

```rust
pub fn wcscasecmp_l_impl(l: &[u32], r: &[u32], _locale: *mut core::ffi::c_void) -> core::ffi::c_int {
    wcscasecmp_impl(l, r)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  wcscasecmp_impl  // 依赖1: 忽略大小写宽字符串比较

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcscasecmp_l(l: *const u32, r: *const u32, locale: *mut core::ffi::c_void) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn wcscasecmp_l_impl(l: &[u32], r: &[u32], locale: *mut core::ffi::c_void) -> core::ffi::c_int;