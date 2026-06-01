# strlcat — Rust 接口归约

## 原始 C 接口
```c
size_t strlcat(char *d, const char *s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strlcat(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> usize;
```

---

## 意图
将字符串 s 追加到大小为 n 的缓冲区 d 中已有字符串之后，始终保证 null 终止（n > 0）。返回所需的总大小。

## 前置条件
- `d` 非空或 `n == 0`
- `s` 非空
- d 以 null 结尾
- s 以 null 结尾

## 后置条件
- 若 n > 0，d[n-1] == '\0'
- 返回 strlen(d 原内容) + strlen(s)
- 若返回值 < n，d 中包含完整拼接结果

## 不变量
- 不会向 d 写入超过 n 字节

## 算法
安全计算 d 在当前缓冲区中的长度后拼接：

```rust
pub fn strlcat_impl(d: &mut [u8], s: &core::ffi::CStr) -> usize {
    let n = d.len();
    let l = d.iter().position(|&b| b == 0).unwrap_or(n);
    if l == n { return n + s.count_bytes(); }
    let space = n - l;
    let src_bytes = s.to_bytes_with_nul();
    let copy = (space - 1).min(src_bytes.len());
    d[l..l + copy].copy_from_slice(&src_bytes[..copy]);
    d[l + copy] = 0;
    l + s.count_bytes()
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes_with_nul  // 依赖1: 含 null 的字节切片
  core::ffi::CStr::count_bytes        // 依赖2: s 的长度
  core::slice::copy_from_slice         // 依赖3: 非重叠复制

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strlcat(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> usize;
Internal Interface:
  pub(crate) fn strlcat_impl(d: &mut [u8], s: &core::ffi::CStr) -> usize;