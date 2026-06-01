# wmemmove — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wmemmove(wchar_t *d, const wchar_t *s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wmemmove(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
```

---

## 意图
将 s 指向的宽字符数组的前 n 个元素复制到 d，正确处理重叠。

## 前置条件
- `d` 非空、`s` 非空
- `d` 至少可写 n 个 wchar_t
- `s` 至少可读 n 个 wchar_t

## 后置条件
- d[0..n-1] == s[0..n-1]（即使重叠也能正确复制）
- 返回 d

## 不变量
- 若 d == s，直接返回
- 重叠时从末尾反向复制

## 算法
使用 ptr::copy 处理重叠：

```rust
pub fn wmemmove_impl(dst: &mut [u32], src: &[u32]) {
    let n = src.len().min(dst.len());
    unsafe { core::ptr::copy(src.as_ptr(), dst.as_mut_ptr(), n); }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::copy  // 依赖1: 重叠安全复制原语

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wmemmove(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
Internal Interface:
  pub(crate) fn wmemmove_impl(dst: &mut [u32], src: &[u32]);