# strncpy — Rust 接口归约

## 原始 C 接口
```c
char *strncpy(char *restrict d, const char *restrict s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strncpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char;
```

---

## 意图
将 s 中最多 n 个字符复制到 d。若 s 长度小于 n，剩余位置用 '\0' 填充。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- `d` 至少可写 n 字节
- s 以 null 结尾

## 后置条件
- d[0..min(strlen(s), n)-1] == s[0..min(strlen(s), n)-1]
- 若 strlen(s) < n，d[strlen(s)..n-1] 全为 '\0'
- 返回 d

## 不变量
- 无全局或静态状态被修改

## 算法
委托给 stpncpy：

```rust
pub fn strncpy_impl(d: &mut [u8], s: &core::ffi::CStr, n: usize) -> *mut u8 {
    let src_bytes = s.to_bytes();
    let copy = n.min(src_bytes.len());
    d[..copy].copy_from_slice(&src_bytes[..copy]);
    if copy < n { d[copy..n].fill(0); }
    unsafe { d.as_mut_ptr() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes      // 依赖1: 字节切片
  core::slice::copy_from_slice   // 依赖2: 非重叠复制
  core::slice::<impl [u8]>::fill // 依赖3: 零填充

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strncpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strncpy_impl(d: &mut [u8], s: &core::ffi::CStr, n: usize) -> *mut u8;