# explicit_bzero — Rust 接口归约

## 原始 C 接口
```c
void explicit_bzero(void *d, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn explicit_bzero(d: *mut core::ffi::c_void, n: usize);
```

---

## 意图
将 d 指向内存的前 n 字节全部置零，并通过编译器屏障阻止优化移除清零操作。用于安全擦除敏感数据。对外导出 C ABI 兼容的 `explicit_bzero` 符号。

## 前置条件
- `d` 非空
- 当 `n > 0` 时，`d` 至少可写 n 字节

## 后置条件
- d[0..n-1] 全部为 0
- 编译器不得优化移除该操作

## 不变量
- 无全局或静态状态被修改

## 算法
调用 memset 后插入编译器屏障。Rust 中使用 `core::sync::atomic::compiler_fence` 或内联汇编实现：

```rust
pub fn explicit_bzero_impl(buf: &mut [u8]) {
    buf.fill(0);
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::fill                            // 依赖1: 填充 slice 为 0
  core::sync::atomic::compiler_fence           // 依赖2: 编译器屏障，等价于 C 的 asm volatile ("" : : "r"(d) : "memory")

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn explicit_bzero(d: *mut core::ffi::c_void, n: usize);
Internal Interface:
  pub(crate) fn explicit_bzero_impl(buf: &mut [u8]);