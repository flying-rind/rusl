# strsep — Rust 接口归约

## 原始 C 接口
```c
char *strsep(char **str, const char *sep);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strsep(str: *mut *mut core::ffi::c_char, sep: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
从 *str 中提取下一个 token，分隔符为 sep 中的任意字符。可正确处理空 token。

## 前置条件
- `str` 非空
- `sep` 非空，以 null 结尾
- 若 *str 非 null，*str 以 null 结尾

## 后置条件
- 若 *str == null，返回 null
- 返回指向当前 token 起始的指针
- 分隔符位置被 '\0' 替换
- *str 更新为下一个 token 起始或 null

## 不变量
- *str 指针单调向前移动

## 算法
使用 strcspn 定位分隔符并替换：

```rust
pub fn strsep_impl(str: &mut Option<&mut [u8]>, sep: &[u8]) -> Option<*mut u8> {
    let s = str.as_mut()?;
    if s.is_empty() || s[0] == 0 { return None; }
    let mut bitset = [0u8; 32];
    for &b in sep { bitset[(b as usize) >> 3] |= 1 << (b & 7); }
    let pos = s.iter().position(|&b| {
        b == 0 || (bitset[(b as usize) >> 3] & (1 << (b & 7))) != 0
    }).unwrap();
    let is_sep = s[pos] != 0;
    if is_sep { s[pos] = 0; }
    let result = s.as_mut_ptr();
    *str = if is_sep { Some(&mut s[pos + 1..]) } else { None };
    Some(result)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::position  // 依赖1: 位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strsep(str: *mut *mut core::ffi::c_char, sep: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strsep_impl(str: &mut Option<&mut [u8]>, sep: &[u8]) -> Option<*mut u8>;