# strchr — Rust 接口归约

## 原始 C 接口
```c
char *strchr(const char *s, int c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strchr(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
```

---

## 意图
在字符串 s 中查找字符 c 首次出现的位置（包括终止 null）。

## 前置条件
- `s` 非空
- s 以 null 结尾

## 后置条件
- 若找到则返回指向匹配位置的指针
- 若未找到则返回 null
- s 内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 strchrnul，若返回地址处的字节不等于 c 则为 null：

```rust
pub fn strchr_impl(s: &core::ffi::CStr, c: u8) -> Option<*const core::ffi::c_char> {
    s.to_bytes().iter().position(|&b| b == c)
        .map(|i| unsafe { s.as_ptr().add(i) })
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes        // 依赖1: 字节切片
  core::iter::Iterator::position   // 依赖2: 位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strchr(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strchr_impl(s: &core::ffi::CStr, c: u8) -> Option<*const core::ffi::c_char>;