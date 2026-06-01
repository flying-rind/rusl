# bcmp — Rust 接口归约

## 原始 C 接口
```c
int bcmp(const void *s1, const void *s2, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn bcmp(s1: *const core::ffi::c_void, s2: *const core::ffi::c_void, n: usize) -> core::ffi::c_int;
```

---

## 意图
比较两个内存区域的前 n 个字节是否相等。对外导出 C ABI 兼容的 `bcmp` 符号供链接器使用。

## 前置条件
- `s1` 非空、`s2` 非空
- 当 `n > 0` 时，`s1` 和 `s2` 各自指向至少可读 n 字节的内存

## 后置条件
- 返回 0 当且仅当前 n 字节完全相等
- 返回非 0 当且仅当存在不相等字节
- 内存内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
内部委托给 `memcmp`。Rust 实现可直接使用 slice 比较：

```rust
pub fn bcmp_impl(s1: &[u8], s2: &[u8]) -> bool {
    s1 == s2
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u8]>::eq  // 依赖1: 字节切片等价比较

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn bcmp(s1: *const core::ffi::c_void, s2: *const core::ffi::c_void, n: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn bcmp_impl(s1: &[u8], s2: &[u8]) -> bool;