# strncasecmp — Rust 接口归约

## 原始 C 接口
```c
int strncasecmp(const char *_l, const char *_r, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strncasecmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char, n: usize) -> core::ffi::c_int;
```

---

## 意图
忽略大小写比较两个 C 字符串的前 n 个字符。

## 前置条件
- `_l` 非空、`_r` 非空
- _l 和 _r 以 null 结尾

## 后置条件
- 若 n == 0，返回 0
- 返回 0：前 min(n, ...) 个字符忽略大小写后相等
- 返回 < 0：首个不同字符处 tolower(l[i]) < tolower(r[i])
- 返回 > 0：首个不同字符处 tolower(l[i]) > tolower(r[i])

## 不变量
- n 递减，不比较超过 n 个字符

## 算法
逐字节比较，经 to_ascii_lowercase 转换：

```rust
pub fn strncasecmp_impl(l: &[u8], r: &[u8], n: usize) -> core::ffi::c_int {
    for (i, (a, b)) in l.iter().zip(r.iter()).enumerate() {
        if i >= n { return 0; }
        let diff = a.to_ascii_lowercase() as i32 - b.to_ascii_lowercase() as i32;
        if diff != 0 || *a == 0 { return diff; }
    }
    0
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  u8::to_ascii_lowercase           // 依赖1: ASCII 小写转换
  core::iter::Iterator::enumerate  // 依赖2: 带索引迭代
  core::iter::Iterator::zip        // 依赖3: 双迭代器合并

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strncasecmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char, n: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn strncasecmp_impl(l: &[u8], r: &[u8], n: usize) -> core::ffi::c_int;