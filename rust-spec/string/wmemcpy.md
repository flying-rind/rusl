# wmemcpy — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wmemcpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wmemcpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
```

---

## 意图
将 s 指向的宽字符数组的前 n 个元素复制到 d。调用者保证不重叠。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- `d` 至少可写 n 个 wchar_t
- `s` 至少可读 n 个 wchar_t

## 后置条件
- d[0..n-1] == s[0..n-1]
- 返回 d

## 不变量
- n 递减确保恰好复制 n 个元素

## 算法
使用 copy_from_slice：

```rust
pub fn wmemcpy_impl(dst: &mut [u32], src: &[u32]) {
    let n = src.len().min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::copy_from_slice  // 依赖1: 非重叠复制

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wmemcpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
Internal Interface:
  pub(crate) fn wmemcpy_impl(dst: &mut [u32], src: &[u32]);