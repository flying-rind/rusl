# wmemcmp — Rust 接口归约

## 原始 C 接口
```c
int wmemcmp(const wchar_t *l, const wchar_t *r, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wmemcmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int;
```

---

## 意图
比较 l 和 r 指向的宽字符数组的前 n 个元素。返回值限定为 -1、0、1。

## 前置条件
- `l` 非空、`r` 非空
- l 和 r 各自至少可读 n 个 wchar_t

## 后置条件
- 返回 0：所有 n 个宽字符相等
- 返回 -1：首个不同处 l[i] < r[i]
- 返回 1：首个不同处 l[i] > r[i]
- 内存内容不变

## 不变量
- n 递减确保不比较超过 n 个元素

## 算法
逐宽字符比较：

```rust
pub fn wmemcmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int {
    for (a, b) in l.iter().zip(r.iter()).take(n) {
        if a != b {
            return if *a < *b { -1 } else { 1 };
        }
    }
    0
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::zip   // 依赖1: 双迭代器合并
  core::iter::Iterator::take  // 依赖2: 限制迭代长度

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wmemcmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn wmemcmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int;