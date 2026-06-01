# wcsnlen — Rust 接口归约

## 原始 C 接口
```c
size_t wcsnlen(const wchar_t *s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsnlen(s: *const u32, n: usize) -> usize;
```

---

## 意图
计算宽字符串 s 的长度，最多搜索 n 个宽字符。

## 前置条件
- `s` 非空
- 当 `n > 0` 时，s 至少可读 min(n, wcslen(s)+1) 个宽字符

## 后置条件
- 若在 n 个字符内找到 L'\0'，返回 wcslen(s)
- 若前 n 个字符均非 L'\0'，返回 n
- s 内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 wmemchr 搜索 L'\0'：

```rust
pub fn wcsnlen_impl(s: &[u32], n: usize) -> usize {
    s.iter().take(n).position(|&c| c == 0).unwrap_or(n)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::take      // 依赖1: 限制迭代长度
  core::iter::Iterator::position  // 依赖2: 查找 null 位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcsnlen(s: *const u32, n: usize) -> usize;
Internal Interface:
  pub(crate) fn wcsnlen_impl(s: &[u32], n: usize) -> usize;