# wcschr — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcschr(const wchar_t *s, wchar_t c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcschr(s: *const u32, c: u32) -> *mut u32;
```

---

## 意图
在宽字符串 s 中查找宽字符 c 首次出现的位置（包括终止 L'\0'）。

## 前置条件
- `s` 非空
- s 以 L'\0' 结尾

## 后置条件
- 若 c == L'\0'，返回指向终止 null 的指针
- 若找到 c，返回指向匹配位置的指针
- 若未找到，返回 null
- s 内容不变

## 不变量
- s 指针单调递增

## 算法
遍历查找宽字符：

```rust
pub fn wcschr_impl(s: &[u32], c: u32) -> Option<*const u32> {
    s.iter().position(|&ch| ch == c).map(|i| unsafe { s.as_ptr().add(i) })
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::position   // 依赖1: 位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcschr(s: *const u32, c: u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcschr_impl(s: &[u32], c: u32) -> Option<*const u32>;