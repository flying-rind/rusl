# bzero — Rust 接口归约

## 原始 C 接口
```c
void bzero(void *s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn bzero(s: *mut core::ffi::c_void, n: usize);
```

---

## 意图
将 s 指向内存的前 n 字节全部置零。对外导出 C ABI 兼容的 `bzero` 符号。

## 前置条件
- `s` 非空
- 当 `n > 0` 时，`s` 至少可写 n 字节

## 后置条件
- s[0..n-1] 全部为 0

## 不变量
- 无全局或静态状态被修改

## 算法
内部委托给 `memset(s, 0, n)`：

```rust
pub fn bzero_impl(buf: &mut [u8]) {
    buf.fill(0);
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::fill  // 依赖1: 填充 slice 为指定值

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn bzero(s: *mut core::ffi::c_void, n: usize);
Internal Interface:
  pub(crate) fn bzero_impl(buf: &mut [u8]);