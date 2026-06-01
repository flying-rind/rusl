# wcsncmp — Rust 接口归约

## 原始 C 接口
```c
int wcsncmp(const wchar_t *l, const wchar_t *r, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsncmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int;
```

---

## 意图
比较两个宽字符串的前 n 个宽字符。返回值限定为 -1、0、1。

## 前置条件
- `l` 非空、`r` 非空
- l 和 r 以 L'\0' 结尾

## 后置条件
- 返回 0：前 n 个字符相等或 n == 0
- 返回 -1：l 字典序小于 r
- 返回 1：l 字典序大于 r
- 字符串内容不变

## 不变量
- n 递减确保不比较超过 n 个字符

## 算法
逐宽字符比较，受限于 n：

```rust
pub fn wcsncmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int {
    for (i, (a, b)) in l.iter().zip(r.iter()).enumerate() {
        if i >= n { return 0; }
        if a != b || *a == 0 {
            return if *a < *b { -1 } else if *a > *b { 1 } else { 0 };
        }
    }
    0
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::zip        // 依赖1: 双迭代器合并
  core::iter::Iterator::enumerate  // 依赖2: 带索引迭代

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcsncmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn wcsncmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int;