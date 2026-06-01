# strrchr — Rust 接口归约

## 原始 C 接口
```c
char *strrchr(const char *s, int c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strrchr(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
```

---

## 意图
在字符串 s 中从后向前查找字符 c 最后一次出现的位置（包括终止 null）。

## 前置条件
- `s` 非空
- s 以 null 结尾

## 后置条件
- 若 c == '\0'，返回指向终止 null 的指针
- 若 c != '\0' 且包含 c，返回 c 最后一次出现位置
- 若 c != '\0' 且不包含 c，返回 null
- s 内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
在包含终止 null 的字节中反向搜索：

```rust
pub fn strrchr_impl(s: &core::ffi::CStr, c: u8) -> *const u8 {
    s.to_bytes_with_nul().iter().rposition(|&b| b == c)
        .map(|i| unsafe { s.as_ptr().add(i) as *const u8 })
        .unwrap_or(core::ptr::null())
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes_with_nul  // 依赖1: 含 null 的字节切片
  core::iter::Iterator::rposition     // 依赖2: 反向位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strrchr(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strrchr_impl(s: &core::ffi::CStr, c: u8) -> *const u8;