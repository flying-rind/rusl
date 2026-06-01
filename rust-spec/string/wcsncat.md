# wcsncat — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcsncat(wchar_t *restrict d, const wchar_t *restrict s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsncat(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
```

---

## 意图
将 s 中最多 n 个宽字符追加到 d 末尾，始终追加 L'\0'。

## 前置条件
- `d` 非空、`s` 非空
- `d` 和 `s` 不重叠
- d 和 s 以 L'\0' 结尾
- d 缓冲区至少可容纳 (wcslen(d) + min(n, wcslen(s)) + 1) 个 wchar_t

## 后置条件
- d 末尾追加了 s 的前 min(n, wcslen(s)) 个宽字符和 L'\0'
- 返回 d

## 不变量
- 最多写入 min(n, wcslen(s)) + 1 个宽字符

## 算法
定位 d 末尾后复制最多 n 个字符：

```rust
pub fn wcsncat_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32 {
    let pos = dst.iter().position(|&c| c == 0).unwrap();
    let src_len = src.iter().position(|&c| c == 0).unwrap();
    let copy = n.min(src_len);
    dst[pos..pos + copy].copy_from_slice(&src[..copy]);
    dst[pos + copy] = 0;
    unsafe { dst.as_mut_ptr() }
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
  extern "C" fn wcsncat(d: *mut u32, s: *const u32, n: usize) -> *mut u32;
Internal Interface:
  pub(crate) fn wcsncat_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32;