# swab — Rust 接口归约

## 原始 C 接口
```c
void swab(const void *restrict _src, void *restrict _dest, ssize_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn swab(_src: *const core::ffi::c_void, _dest: *mut core::ffi::c_void, n: isize);
```

---

## 意图
将 _src 中的 n 个字节相邻两两交换后复制到 _dest。若 n 为奇数，最后 1 字节不处理。

## 前置条件
- `_src` 非空、`_dest` 非空
- `_src` 和 `_dest` 不重叠
- `_src` 至少可读 n 字节
- `_dest` 至少可写 (n & !1) 字节

## 后置条件
- dest[2*i] == src[2*i+1] 且 dest[2*i+1] == src[2*i]
- _src 内容不变

## 不变量
- 每对字节处理后 n 递减 2

## 算法
成对交换字节：

```rust
pub fn swab_impl(src: &[u8], dst: &mut [u8], n: usize) {
    let pairs = (n as usize) / 2;
    for i in 0..pairs {
        dst[2 * i] = src[2 * i + 1];
        dst[2 * i + 1] = src[2 * i];
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u8]>::len  // 依赖1: 切片长度

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn swab(_src: *const core::ffi::c_void, _dest: *mut core::ffi::c_void, n: isize);
Internal Interface:
  pub(crate) fn swab_impl(src: &[u8], dst: &mut [u8], n: usize);