# wcslen — Rust 接口归约

## 原始 C 接口
```c
size_t wcslen(const wchar_t *s);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcslen(s: *const u32) -> usize;
```

---

## 意图
计算宽字符串 s 的长度（不含终止 L'\0'）。

## 前置条件
- `s` 非空
- s 以 L'\0' 结尾

## 后置条件
- 返回满足 s[i] == L'\0' 的最小 i
- s 内容不变

## 不变量
- s 指针单调递增直到遇到 L'\0'

## 算法
逐宽字符遍历到 L'\0'：

```rust
pub fn wcslen_impl(s: &[u32]) -> usize {
    s.iter().position(|&c| c == 0).unwrap_or(s.len())
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::position  // 依赖1: 查找 null 位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcslen(s: *const u32) -> usize;
Internal Interface:
  pub(crate) fn wcslen_impl(s: &[u32]) -> usize;