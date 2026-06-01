# wcspbrk — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcspbrk(const wchar_t *s, const wchar_t *b);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcspbrk(s: *const u32, b: *const u32) -> *mut u32;
```

---

## 意图
在宽字符串 s 中查找 b 中任意宽字符首次出现的位置。

## 前置条件
- `s` 非空、`b` 非空
- s 和 b 以 L'\0' 结尾

## 后置条件
- 若找到 b 中的任意宽字符，返回指向该位置的指针
- 若未找到，返回 null
- s 和 b 内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
使用 wcscspn 计算偏移量：

```rust
pub fn wcspbrk_impl(s: &[u32], accept: &[u32]) -> Option<*const u32> {
    let pos = wcscspn_impl(s, accept);
    if pos < s.len() && s[pos] != 0 { Some(unsafe { s.as_ptr().add(pos) }) } else { None }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  wcscspn_impl  // 依赖1: 计算不含 accept 字符的前缀长度

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcspbrk(s: *const u32, b: *const u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcspbrk_impl(s: &[u32], accept: &[u32]) -> Option<*const u32>;