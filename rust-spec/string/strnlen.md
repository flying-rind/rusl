# strnlen — Rust 接口归约

## 原始 C 接口
```c
size_t strnlen(const char *s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strnlen(s: *const core::ffi::c_char, n: usize) -> usize;
```

---

## 意图
计算字符串 s 的长度，最多搜索 n 个字符。若在 n 个字符内未找到 '\0'，返回 n。

## 前置条件
- `s` 非空
- 当 `n > 0` 时，s 至少可读 min(n, strlen(s)+1) 字节

## 后置条件
- 若 s 的 '\0' 在 n 字节内，返回 strlen(s)
- 若前 n 字节均非 '\0'，返回 n
- s 内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 memchr 搜索 '\0'：

```rust
pub fn strnlen_impl(s: &[u8], n: usize) -> usize {
    s.iter().take(n).position(|&b| b == 0).unwrap_or(n)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::take      // 依赖1: 限制迭代长度
  core::iter::Iterator::position  // 依赖2: 位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strnlen(s: *const core::ffi::c_char, n: usize) -> usize;
Internal Interface:
  pub(crate) fn strnlen_impl(s: &[u8], n: usize) -> usize;