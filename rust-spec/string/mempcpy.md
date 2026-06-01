# mempcpy — Rust 接口归约

## 原始 C 接口
```c
void *mempcpy(void *dest, const void *src, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn mempcpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void;
```

---

## 意图
将 src 的前 n 字节复制到 dest，返回 dest + n（最后一个写入字节之后的位置）。

## 前置条件
- `dest` 非空、`src` 非空
- `dest` 至少可写 n 字节，`src` 至少可读 n 字节
- `dest` 和 `src` 不重叠

## 后置条件
- dest[0..n-1] == src[0..n-1]
- 返回 dest + n

## 不变量
- 无全局或静态状态被修改

## 算法
内部委托给 memcpy，返回 dest + n：

```rust
pub fn mempcpy_impl(dst: &mut [u8], src: &[u8]) -> &mut [u8] {
    let n = src.len();
    dst[..n].copy_from_slice(src);
    &mut dst[n..]
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::copy_from_slice  // 依赖1: 非重叠内存复制

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn mempcpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn mempcpy_impl(dst: &mut [u8], src: &[u8]) -> &mut [u8];