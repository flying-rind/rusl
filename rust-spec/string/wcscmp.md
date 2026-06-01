# wcscmp — Rust 接口归约

## 原始 C 接口
```c
int wcscmp(const wchar_t *l, const wchar_t *r);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcscmp(l: *const u32, r: *const u32) -> core::ffi::c_int;
```

---

## 意图
比较两个宽字符串 l 和 r。

## 前置条件
- `l` 非空、`r` 非空
- l 和 r 以 L'\0' 结尾

## 后置条件
- 返回 0：字符串完全相同
- 返回 -1：l 字典序小于 r
- 返回 1：l 字典序大于 r
- 字符串内容不变

## 不变量
- l 和 r 指针增量相等

## 算法
逐宽字符比较，返回值限定为 -1/0/1：

```rust
pub fn wcscmp_impl(l: &[u32], r: &[u32]) -> core::ffi::c_int {
    for (a, b) in l.iter().zip(r.iter()) {
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
  core::iter::Iterator::zip  // 依赖1: 双迭代器合并

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcscmp(l: *const u32, r: *const u32) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn wcscmp_impl(l: &[u32], r: &[u32]) -> core::ffi::c_int;