# wcscasecmp — Rust 接口归约

## 原始 C 接口
```c
int wcscasecmp(const wchar_t *l, const wchar_t *r);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcscasecmp(l: *const u32, r: *const u32) -> core::ffi::c_int;
```

---

## 意图
忽略大小写比较两个宽字符串。

## 前置条件
- `l` 非空、`r` 非空
- l 和 r 以 L'\0' 结尾

## 后置条件
- 返回 0：忽略大小写后相等
- 返回 < 0：l 字典序小于 r
- 返回 > 0：l 字典序大于 r
- 字符串内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 wcsncasecmp，用 usize::MAX 作为长度上限：

```rust
pub fn wcscasecmp_impl(l: &[u32], r: &[u32]) -> core::ffi::c_int {
    wcsncasecmp_impl(l, r, usize::MAX)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  wcsncasecmp_impl  // 依赖1: 定长忽略大小写宽字符串比较

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcscasecmp(l: *const u32, r: *const u32) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn wcscasecmp_impl(l: &[u32], r: &[u32]) -> core::ffi::c_int;