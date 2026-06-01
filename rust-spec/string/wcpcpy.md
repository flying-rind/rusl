# wcpcpy — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcpcpy(wchar_t *restrict d, const wchar_t *restrict s);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcpcpy(d: *mut u32, s: *const u32) -> *mut u32;
```

---

## 意图
将 s 指向的宽字符串（含终止 L'\0'）复制到 d，返回 d 中终止 null 的位置。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- `d` 至少可写 (wcslen(s) + 1) 个 wchar_t
- s 以 L'\0' 结尾

## 后置条件
- d[0..len] == s[0..len]（len == wcslen(s)），含终止 null
- 返回 &d[len]

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 wcscpy 和 wcslen：

```rust
pub fn wcpcpy_impl(dst: &mut [u32], src: &[u32]) -> *mut u32 {
    let len = src.iter().position(|&c| c == 0).unwrap();
    dst[..=len].copy_from_slice(&src[..=len]);
    unsafe { dst.as_mut_ptr().add(len) }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::copy_from_slice    // 依赖1: 非重叠复制
  core::iter::Iterator::position  // 依赖2: 查找 null 位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcpcpy(d: *mut u32, s: *const u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcpcpy_impl(dst: &mut [u32], src: &[u32]) -> *mut u32;