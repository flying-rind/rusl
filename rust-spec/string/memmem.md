# memmem — Rust 接口归约

## 原始 C 接口
```c
void *memmem(const void *h0, size_t k, const void *n0, size_t l);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn memmem(h0: *const core::ffi::c_void, k: usize, n0: *const core::ffi::c_void, l: usize) -> *mut core::ffi::c_void;
```

---

## 意图
在长度为 k 的内存区域（haystack）h0 中查找长度为 l 的子序列（needle）n0 第一次出现的位置。

## 前置条件
- `h0` 非空或 `k == 0`
- `n0` 非空或 `l == 0`
- 当 `k > 0` 时，`h0` 至少可读 k 字节
- 当 `l > 0` 时，`n0` 至少可读 l 字节

## 后置条件
- 若 l == 0，返回 h0
- 若 k < l，返回 null
- 若找到匹配子序列，返回指向 h0 中首次匹配位置的指针
- 若未找到，返回 null

## 不变量
- 搜索窗口始终在 h0[0..k-1] 区间内
- Two-Way 算法中 shift 表、byteset 在搜索期间不变

## 算法
Rust 实现可使用 `[u8]::windows` 或手写 Two-Way 算法：

```rust
pub fn memmem_impl(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() { return Some(0); }
    haystack.windows(needle.len()).position(|w| w == needle)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u8]>::windows  // 依赖1: 滑动窗口遍历
  core::slice::<impl [u8]>::eq       // 依赖2: 切片等价比较
  core::option::Option               // 依赖3: 可选返回值

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn memmem(h0: *const core::ffi::c_void, k: usize, n0: *const core::ffi::c_void, l: usize) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn memmem_impl(haystack: &[u8], needle: &[u8]) -> Option<usize>;