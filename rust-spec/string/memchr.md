# memchr — Rust 接口归约

## 原始 C 接口
```c
void *memchr(const void *src, int c, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn memchr(src: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
```

---

## 意图
在 src 指向内存的前 n 字节中查找字符 c 第一次出现的位置。对外导出 C ABI 兼容的 `memchr` 符号。

## 前置条件
- `src` 非空
- 当 `n > 0` 时，`src` 至少可读 n 字节

## 后置条件
- 若找到 c，返回指向该位置的指针
- 若未找到，返回 null
- src 内存内容不变

## 不变量
- 搜索位置从未越界

## 算法
逐字节或字级别搜索。Rust 中可使用 `position`：

```rust
pub fn memchr_impl(buf: &[u8], c: u8) -> Option<usize> {
    buf.iter().position(|&b| b == c)
}
```

若需性能优化，可使用手写字级别搜索，利用 `usize` 的位操作模拟 HASZERO 宏。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::iter::position  // 依赖1: 查找字节位置

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn memchr(src: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn memchr_impl(buf: &[u8], c: u8) -> Option<usize>;