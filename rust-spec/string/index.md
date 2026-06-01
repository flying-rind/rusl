# index — Rust 接口归约

## 原始 C 接口
```c
char *index(const char *s, int c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn index(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
```

---

## 意图
在字符串 s 中查找字符 c 第一次出现的位置。对外导出 C ABI 兼容的 `index` 符号。

## 前置条件
- `s` 非空，指向以 null 结尾的有效 C 字符串

## 后置条件
- 若存在 i 使得 s[i] == (char)c，返回 &s[i]
- 若不存在，返回 NULL
- s 内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
内部委托给 `strchr`：

```rust
pub fn index_impl(s: &core::ffi::CStr, c: u8) -> Option<*const u8> {
    s.to_bytes().iter().position(|&b| b == c).map(|i| &s.to_bytes()[i] as *const u8)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes  // 依赖1: 获取 C 字符串的字节切片
  core::slice::iter::position  // 依赖2: 查找字节位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn index(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn index_impl(s: &core::ffi::CStr, c: u8) -> Option<*const u8>;