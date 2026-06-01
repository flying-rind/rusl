# wcsrchr — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcsrchr(const wchar_t *s, wchar_t c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsrchr(s: *const u32, c: u32) -> *mut u32;
```

---

## 意图
在宽字符串 s 中从后向前查找宽字符 c 最后一次出现的位置（包括终止 L'\0'）。

## 前置条件
- `s` 非空
- s 以 L'\0' 结尾

## 后置条件
- 若找到 c，返回指向最后一次匹配位置的指针
- 若未找到，返回 null
- s 内容不变

## 不变量
- 指针从字符串末尾向开头递减搜索

## 算法
从末尾反向搜索：

```rust
pub fn wcsrchr_impl(s: &[u32], c: u32) -> Option<*const u32> {
    let len = s.iter().position(|&ch| ch == 0).unwrap_or(s.len());
    s[..=len].iter().rposition(|&ch| ch == c)
        .map(|i| unsafe { s.as_ptr().add(i) })
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::position   // 依赖1: 查找 null 位置
  core::iter::Iterator::rposition  // 依赖2: 反向位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcsrchr(s: *const u32, c: u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcsrchr_impl(s: &[u32], c: u32) -> Option<*const u32>;