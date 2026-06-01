# memcmp — Rust 接口归约

## 原始 C 接口
```c
int memcmp(const void *vl, const void *vr, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn memcmp(vl: *const core::ffi::c_void, vr: *const core::ffi::c_void, n: usize) -> core::ffi::c_int;
```

---

## 意图
比较 vl 和 vr 指向内存区域的前 n 个字节。对外导出 C ABI 兼容的 `memcmp` 符号。

## 前置条件
- `vl` 非空、`vr` 非空
- 当 `n > 0` 时，两者各自至少可读 n 字节

## 后置条件
- 返回 0：全部相等
- 返回 <0：在第一个不等字节处 vl[i] < vr[i]
- 返回 >0：在第一个不等字节处 vl[i] > vr[i]
- 内存内容不变

## 不变量
- 比较字节数从未超过 n

## 算法
逐字节比较，在第一个不相等处返回差值：

```rust
pub fn memcmp_impl(a: &[u8], b: &[u8]) -> core::cmp::Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        match x.cmp(y) {
            core::cmp::Ordering::Equal => continue,
            ne => return ne,
        }
    }
    core::cmp::Ordering::Equal
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::cmp::Ord::cmp    // 依赖1: 字节比较
  core::iter::Iterator::zip  // 依赖2: 并行迭代

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn memcmp(vl: *const core::ffi::c_void, vr: *const core::ffi::c_void, n: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn memcmp_impl(a: &[u8], b: &[u8]) -> core::cmp::Ordering;