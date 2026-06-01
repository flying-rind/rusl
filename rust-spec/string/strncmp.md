# strncmp — Rust 接口归约

## 原始 C 接口
```c
int strncmp(const char *_l, const char *_r, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strncmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char, n: usize) -> core::ffi::c_int;
```

---

## 意图
比较两个 C 字符串的前 n 个字符。

## 前置条件
- `_l` 非空、`_r` 非空
- _l 和 _r 以 null 结尾

## 后置条件
- 若 n == 0，返回 0
- 返回 0：前 min(n, ...) 个字符相等
- 返回 < 0：首个不同字符处 l[i] < r[i]（作为 u8）
- 返回 > 0：首个不同字符处 l[i] > r[i]

## 不变量
- n 递减确保不比较超过 n 个字符

## 算法
逐字节比较，受限于 n：

```rust
pub fn strncmp_impl(l: &[u8], r: &[u8], n: usize) -> core::ffi::c_int {
    for (i, (a, b)) in l.iter().zip(r.iter()).enumerate() {
        if i >= n { return 0; }
        if a != b || *a == 0 { return (*a as i32) - (*b as i32); }
    }
    if l.len().min(r.len()) < n { 0 } else { 0 }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::zip       // 依赖1: 双迭代器合并
  core::iter::Iterator::enumerate // 依赖2: 带索引迭代

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strncmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char, n: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn strncmp_impl(l: &[u8], r: &[u8], n: usize) -> core::ffi::c_int;