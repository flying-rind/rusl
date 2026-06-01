# memrchr — Rust 接口归约

## 原始 C 接口
```c
void *memrchr(const void *m, int c, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn memrchr(m: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
```

---

## 意图
在 m 指向内存区域的前 n 字节中从后向前查找字符 c 最后一次出现的位置。

## 前置条件
- `m` 非空
- 当 `n > 0` 时，`m` 至少可读 n 字节

## 后置条件
- 若找到则返回指向最后一次匹配位置的指针
- 若未找到则返回 null
- m 内容不变

## 不变量
- 搜索从索引 n-1 向 0 递减

## 算法
从末尾向开头遍历：

```rust
pub fn memrchr_impl(buf: &[u8], byte: u8) -> Option<usize> {
    buf.iter().rposition(|&b| b == byte)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u8]>::iter   // 依赖1: 字节切片迭代器
  core::iter::Iterator::rposition  // 依赖2: 反向位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn memrchr(m: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn memrchr_impl(buf: &[u8], byte: u8) -> Option<usize>;