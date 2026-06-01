# wcsncasecmp — Rust 接口归约

## 原始 C 接口
```c
int wcsncasecmp(const wchar_t *l, const wchar_t *r, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsncasecmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int;
```

---

## 意图
忽略大小写比较两个宽字符串的前 n 个宽字符。

## 前置条件
- `l` 非空、`r` 非空
- l 和 r 以 L'\0' 结尾

## 后置条件
- 若 n == 0，返回 0
- 返回 0：前 min(n, ...) 个字符忽略大小写后相等
- 返回 < 0：首个不同字符处 towlower(l[i]) < towlower(r[i])
- 返回 > 0：首个不同字符处 towlower(l[i]) > towlower(r[i])

## 不变量
- n 递减确保不比较超过 n 个字符

## 算法
逐宽字符比较，通过 towlower 转换：

```rust
pub fn wcsncasecmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int {
    for (i, (a, b)) in l.iter().zip(r.iter()).enumerate() {
        if i >= n { return 0; }
        let la = towlower_impl(*a);
        let lb = towlower_impl(*b);
        if la != lb || *a == 0 { return if la < lb { -1 } else if la > lb { 1 } else { 0 }; }
    }
    0
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  towlower_impl                    // 依赖1: 宽字符转小写
  core::iter::Iterator::zip        // 依赖2: 双迭代器合并
  core::iter::Iterator::enumerate  // 依赖3: 带索引迭代

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcsncasecmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn wcsncasecmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int;