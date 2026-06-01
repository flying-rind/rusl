# stpncpy — Rust 接口归约

## 原始 C 接口
```c
char *stpncpy(char *restrict d, const char *restrict s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn stpncpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char;
```

---

## 意图
将 s 中最多 n 个字符复制到 d。若 s 长度小于 n，剩余用 '\0' 填充。返回写入的最后一个非 null 字符之后的位置。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- `d` 至少可写 n 字节
- s 以 null 结尾

## 后置条件
- d[0..k-1] == s[0..k-1]（k == min(strlen(s), n)）
- 若 strlen(s) < n，d[k..n-1] == 0
- 返回 d + k

## 不变量
- 剩余数量 n 持续递减

## 算法
先复制不超过 n 的非 null 字符，剩余用 0 填充：

```rust
pub fn stpncpy_impl(dst: &mut [u8], src: &core::ffi::CStr, n: usize) -> *mut u8 {
    let src_bytes = src.to_bytes();
    let k = src_bytes.len().min(n);
    dst[..k].copy_from_slice(&src_bytes[..k]);
    dst[k..n].fill(0);
    unsafe { dst.as_mut_ptr().add(k) }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes      // 依赖1: 不含 null 的字节切片
  core::slice::copy_from_slice   // 依赖2: 非重叠复制
  core::slice::<impl [u8]>::fill // 依赖3: 零填充

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn stpncpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn stpncpy_impl(dst: &mut [u8], src: &core::ffi::CStr, n: usize) -> *mut u8;