# wmemset — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wmemset(wchar_t *d, wchar_t c, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wmemset(d: *mut u32, c: u32, n: usize) -> *mut u32;
```

---

## 意图
将 d 指向的宽字符数组的前 n 个元素全部设置为宽字符 c。

## 前置条件
- `d` 非空
- `d` 至少可写 n 个 wchar_t

## 后置条件
- d[0..n-1] 全部为 c
- 返回 d

## 不变量
- n 递减确保恰好设置 n 个元素

## 算法
使用 slice::fill：

```rust
pub fn wmemset_impl(dst: &mut [u32], c: u32) {
    dst[..dst.len()].fill(c);
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u32]>::fill  // 依赖1: 切片填充原语

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wmemset(d: *mut u32, c: u32, n: usize) -> *mut u32;
Internal Interface:
  pub(crate) fn wmemset_impl(dst: &mut [u32], c: u32);