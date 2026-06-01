# strcmp — Rust 接口归约

## 原始 C 接口
```c
int strcmp(const char *l, const char *r);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strcmp(l: *const core::ffi::c_char, r: *const core::ffi::c_char) -> core::ffi::c_int;
```

---

## 意图
比较两个 C 字符串 l 和 r 的字典序大小。

## 前置条件
- `l` 非空、`r` 非空
- l 和 r 以 null 结尾

## 后置条件
- 返回 0：两字符串完全相等
- 返回 < 0：首个不同字符处 l[i] < r[i]（作为 u8）
- 返回 > 0：首个不同字符处 l[i] > r[i]（作为 u8）
- 字符串内容不变

## 不变量
- 循环在首个不同字符处或同时到达 null 时终止

## 算法
逐字节比较，直到遇到不同或 null：

```rust
pub fn strcmp_impl(l: &core::ffi::CStr, r: &core::ffi::CStr) -> core::ffi::c_int {
    for (a, b) in l.to_bytes().iter().zip(r.to_bytes().iter()) {
        if a != b { return (*a as i32) - (*b as i32); }
    }
    (l.count_bytes() as i32) - (r.count_bytes() as i32)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes     // 依赖1: 字节切片
  core::ffi::CStr::count_bytes  // 依赖2: 字符串长度

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strcmp(l: *const core::ffi::c_char, r: *const core::ffi::c_char) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn strcmp_impl(l: &core::ffi::CStr, r: &core::ffi::CStr) -> core::ffi::c_int;