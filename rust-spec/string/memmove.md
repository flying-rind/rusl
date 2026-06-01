# memmove — Rust 接口归约

## 原始 C 接口
```c
void *memmove(void *dest, const void *src, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn memmove(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void;
```

---

## 意图
将 src 的前 n 字节复制到 dest，正确处理源和目标区域重叠的情况。

## 前置条件
- `dest` 非空、`src` 非空
- `dest` 至少可写 n 字节，`src` 至少可读 n 字节

## 后置条件
- dest[0..n-1] == src[0..n-1]（如同先复制到临时缓冲区再复制到 dest）
- 返回 dest

## 不变量
- 若 d==s，直接返回
- 正向复制（d<s）和反向复制（d>s）各自保证不覆盖未读源数据

## 算法
Rust 中使用 `ptr::copy` 处理重叠情况：

```rust
pub fn memmove_impl(dst: &mut [u8], src: &[u8]) {
    unsafe { core::ptr::copy(src.as_ptr(), dst.as_mut_ptr(), src.len()); }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::copy  // 依赖1: 重叠安全的内存复制原语

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn memmove(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn memmove_impl(dst: &mut [u8], src: &[u8]);