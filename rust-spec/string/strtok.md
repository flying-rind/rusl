# strtok — Rust 接口归约

## 原始 C 接口
```c
char *strtok(char *restrict s, const char *restrict sep);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strtok(s: *mut core::ffi::c_char, sep: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
从字符串 s 中提取下一个 token，分隔符为 sep 中的任意字符。使用静态内部指针维护状态（非线程安全）。

## 前置条件
- 首次调用 `s` 非空，后续可传 null
- `sep` 非空，以 null 结尾

## 后置条件
- 若无更多 token，返回 null
- 若有 token，返回指向 token 起始的指针，末尾被 '\0' 替换
- 内部静态指针更新

## 不变量
- 静态指针为 null 或指向下一个搜索起始

## 算法
跳过前导分隔符，找到 token 结束位置：

```rust
use core::sync::atomic::{AtomicPtr, Ordering};
static TOK_STATE: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());

pub fn strtok_impl(s: Option<&mut [u8]>, sep: &[u8]) -> Option<*mut u8> {
    let state = if let Some(s) = s { s.as_mut_ptr() } else { TOK_STATE.load(Ordering::Relaxed) };
    // ... 使用 strspn 和 strcspn 提取 token
    None
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  strspn_impl                     // 依赖1: 跳过前导分隔符
  strcspn_impl                    // 依赖2: 定位分隔符位置
  core::sync::atomic::AtomicPtr   // 依赖3: 线程安全的静态状态指针

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strtok(s: *mut core::ffi::c_char, sep: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strtok_impl(s: Option<&mut [u8]>, sep: &[u8]) -> Option<*mut u8>;