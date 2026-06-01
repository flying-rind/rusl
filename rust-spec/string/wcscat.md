# wcscat — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcscat(wchar_t *restrict dest, const wchar_t *restrict src);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcscat(dest: *mut u32, src: *const u32) -> *mut u32;
```

---

## 意图
将 src 宽字符串追加到 dest 宽字符串末尾。

## 前置条件
- `dest` 非空、`src` 非空
- `dest` 和 `src` 不重叠
- dest 和 src 以 L'\0' 结尾
- dest 缓冲区至少可容纳 (wcslen(dest) + wcslen(src) + 1) 个 wchar_t

## 后置条件
- dest 末尾追加了 src 内容
- 返回 dest

## 不变量
- 无全局或静态状态被修改

## 算法
定位 dest 末尾后复制 src：

```rust
pub fn wcscat_impl(dest: &mut [u32], src: &[u32]) -> *mut u32 {
    let pos = dest.iter().position(|&c| c == 0).unwrap();
    let src_len = src.iter().position(|&c| c == 0).unwrap();
    dest[pos..=pos + src_len].copy_from_slice(&src[..=src_len]);
    unsafe { dest.as_mut_ptr() }
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
  extern "C" fn wcscat(dest: *mut u32, src: *const u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcscat_impl(dest: &mut [u32], src: &[u32]) -> *mut u32;