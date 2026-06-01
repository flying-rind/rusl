# wcsncpy — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcsncpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
```

---

## 意图
将 s 中最多 n 个宽字符复制到 d。若 wcslen(s) < n，剩余用 L'\0' 填充。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- `d` 至少可写 n 个 wchar_t
- s 以 L'\0' 结尾

## 后置条件
- d[0..k-1] == s[0..k-1]（k == min(wcslen(s), n)）
- 若 wcslen(s) < n，剩余填充 L'\0'
- 返回 d

## 不变量
- 复制停止于 n 耗尽或遇到 L'\0'

## 算法
复制非 null 字符后填充剩余：

```rust
pub fn wcsncpy_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32 {
    let src_len = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    let copy = n.min(src_len);
    dst[..copy].copy_from_slice(&src[..copy]);
    if copy < n { dst[copy..n].fill(0); }
    unsafe { dst.as_mut_ptr() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::copy_from_slice     // 依赖1: 非重叠复制
  core::slice::<impl [u32]>::fill  // 依赖2: 零填充
  core::iter::Iterator::position   // 依赖3: 查找 null 位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcsncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
Internal Interface:
  pub(crate) fn wcsncpy_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32;