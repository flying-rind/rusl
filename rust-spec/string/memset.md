# memset — Rust 接口归约

## 原始 C 接口
```c
void *memset(void *dest, int c, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn memset(dest: *mut core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
```

---

## 意图
将 dest 的前 n 字节全部设置为值 c（转为 u8）。

## 前置条件
- `dest` 非空
- `dest` 至少可写 n 字节

## 后置条件
- 对于所有 i，0 <= i < n，dest[i] == (c as u8)
- 返回 dest

## 不变量
- 填充分阶段：头部对齐、批量填充、尾部处理

## 算法
Rust 实现使用 `slice::fill`：

```rust
pub fn memset_impl(dst: &mut [u8], val: u8) {
    dst.fill(val);
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u8]>::fill  // 依赖1: 切片填充原语

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn memset(dest: *mut core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
Internal Interface:
  pub(crate) fn memset_impl(dst: &mut [u8], val: u8);