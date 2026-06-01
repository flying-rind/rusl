# strlcpy — Rust 接口归约

## 原始 C 接口
```c
size_t strlcpy(char *d, const char *s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strlcpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> usize;
```

---

## 意图
将字符串 s 复制到大小为 n 的缓冲区 d 中，始终保证 null 终止（n > 0）。返回 strlen(s)。

## 前置条件
- `d` 非空或 `n == 0`
- `s` 非空
- `d` 至少可写 n 字节（n > 0 时）
- s 以 null 结尾

## 后置条件
- 若 n > 0，d[0..k-1] == s[0..k-1] 且 d[k-1] == '\0'（k = min(n, strlen(s)+1)）
- 返回 strlen(s)

## 不变量
- 最多写入 n 字节到 d

## 算法
安全复制字符串到固定大小缓冲区：

```rust
pub fn strlcpy_impl(d: &mut [u8], s: &core::ffi::CStr) -> usize {
    let n = d.len();
    let src_bytes = s.to_bytes();
    let src_len = src_bytes.len();
    if n == 0 { return src_len; }
    let copy = (n - 1).min(src_len);
    d[..copy].copy_from_slice(&src_bytes[..copy]);
    d[copy] = 0;
    src_len
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes       // 依赖1: 字节切片
  core::slice::copy_from_slice    // 依赖2: 非重叠复制

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strlcpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> usize;
Internal Interface:
  pub(crate) fn strlcpy_impl(d: &mut [u8], s: &core::ffi::CStr) -> usize;