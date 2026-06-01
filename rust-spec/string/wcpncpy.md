# wcpncpy — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcpncpy(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcpncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
```

---

## 意图
将 s 中最多 n 个宽字符复制到 d。若 s 长度小于 n，剩余用 L'\0' 填充。返回 d + min(wcslen(s), n)。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- `d` 至少可写 n 个 wchar_t
- s 以 L'\0' 结尾

## 后置条件
- d[0..k-1] == s[0..k-1]（k == min(wcslen(s), n)）
- 若 wcslen(s) < n，d[k..n-1] 全为 0
- 返回 d + k

## 不变量
- 无全局或静态状态被修改

## 算法
复制最多 n 个字符，填充剩余：

```rust
pub fn wcpncpy_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32 {
    let k = src.iter().position(|&c| c == 0).unwrap_or(src.len()).min(n);
    dst[..k].copy_from_slice(&src[..k]);
    if k < n { dst[k..n].fill(0); }
    unsafe { dst.as_mut_ptr().add(k) }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::copy_from_slice    // 依赖1: 非重叠复制
  core::slice::<impl [u32]>::fill // 依赖2: 零填充
  core::iter::Iterator::position  // 依赖3: 查找 null 位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcpncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
Internal Interface:
  pub(crate) fn wcpncpy_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32;