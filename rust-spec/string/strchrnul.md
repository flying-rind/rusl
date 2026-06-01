# strchrnul — Rust 接口归约

## 原始 C 接口
```c
char *strchrnul(const char *s, int c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strchrnul(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
```

---

## 意图
在字符串 s 中查找字符 c 首次出现的位置。若未找到，返回指向终止 null 的指针。

## 前置条件
- `s` 非空
- s 以 null 结尾

## 后置条件
- 若找到 c，返回指向匹配位置的指针
- 若未找到，返回指向 s 末尾 '\0' 的指针
- s 内容不变

## 不变量
- 搜索位置在 s[0..strlen(s)] 区间内

## 算法
逐字节搜索，同时检测 '\0' 和目标字符：

```rust
pub fn strchrnul_impl(s: &core::ffi::CStr, c: u8) -> *const u8 {
    s.to_bytes_with_nul().iter()
        .position(|&b| b == c)
        .map(|i| unsafe { s.as_ptr().add(i) as *const u8 })
        .unwrap_or(unsafe { s.as_ptr().add(s.count_bytes()) as *const u8 })
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes_with_nul  // 依赖1: 含 null 的字节切片
  core::iter::Iterator::position      // 依赖2: 位置查找
  core::ffi::CStr::count_bytes        // 依赖3: 字符串长度

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strchrnul(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strchrnul_impl(s: &core::ffi::CStr, c: u8) -> *const u8;