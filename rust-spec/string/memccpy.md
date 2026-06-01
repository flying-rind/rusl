# memccpy — Rust 接口归约

## 原始 C 接口
```c
void *memccpy(void *restrict dest, const void *restrict src, int c, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn memccpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
```

---

## 意图
从 src 复制字节到 dest，直到已复制 n 字节或遇到字符 c。若遇到 c，将其复制后停止，返回 dest 中 c 之后的下一个字节位置。对外导出 C ABI 兼容的 `memccpy` 符号。

## 前置条件
- `dest` 非空、`src` 非空
- `dest` 和 `src` 不重叠（restrict 约束）
- `dest` 至少可写 n 字节，`src` 至少可读 n 字节

## 后置条件
- 若在 n 字节内遇到 c：dest[0..k-1] == src[0..k-1]（src[k-1]==c, k<=n），返回 dest+k
- 若未遇到 c：dest[0..n-1] == src[0..n-1]，返回 null

## 不变量
- 复制偏移量不超过 n

## 算法
逐字节复制并在遇到目标字符时提前返回。Rust 中可使用迭代器实现：

```rust
pub fn memccpy_impl(dst: &mut [u8], src: &[u8], c: u8) -> Option<&mut u8> {
    for (i, &b) in src.iter().enumerate().take(dst.len()) {
        dst[i] = b;
        if b == c {
            return Some(&mut dst[i + 1]);
        }
    }
    None
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::iter::enumerate  // 依赖1: 索引迭代
  core::iter::Iterator::take    // 依赖2: 限制迭代次数

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn memccpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn memccpy_impl(dst: &mut [u8], src: &[u8], c: u8) -> Option<&mut u8>;