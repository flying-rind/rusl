# wmemchr — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wmemchr(const wchar_t *s, wchar_t c, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wmemchr(s: *const u32, c: u32, n: usize) -> *mut u32;
```

---

## 意图
在 s 指向的宽字符数组的前 n 个元素中查找宽字符 c 首次出现的位置。

## 前置条件
- `s` 非空
- `s` 至少可读 n 个 wchar_t

## 后置条件
- 若找到 c，返回指向匹配位置的指针
- 若未找到，返回 null
- s 内容不变

## 不变量
- n 递减确保不搜索超过 n 个元素

## 算法
逐元素遍历：

```rust
pub fn wmemchr_impl(s: &[u32], c: u32, n: usize) -> Option<*const u32> {
    s.iter().take(n).position(|&ch| ch == c)
        .map(|i| unsafe { s.as_ptr().add(i) })
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
  extern "C" fn wmemchr(s: *const u32, c: u32, n: usize) -> *mut u32;
Internal Interface:
  pub(crate) fn wmemchr_impl(s: &[u32], c: u32, n: usize) -> Option<*const u32>;