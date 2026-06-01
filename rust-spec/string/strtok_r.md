# strtok_r — Rust 接口归约

## 原始 C 接口
```c
char *strtok_r(char *restrict s, const char *restrict sep, char **restrict p);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strtok_r(s: *mut core::ffi::c_char, sep: *const core::ffi::c_char, p: *mut *mut core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
strtok 的可重入版本。从字符串 s 中提取下一个 token，使用调用者提供的指针 *p 维护状态。

## 前置条件
- `p` 非空
- 首次调用 `s` 非空，后续可传 null
- `sep` 非空，以 null 结尾

## 后置条件
- 若无更多 token，返回 null，*p 设为 null
- 若有 token，返回 token 起始指针，*p 更新为下一搜索位置

## 不变量
- *p 为 null 或指向下一搜索起始位置

## 算法
使用 strspn 和 strcspn 提取 token：

```rust
pub fn strtok_r_impl(s: Option<&mut [u8]>, sep: &[u8], state: &mut Option<*mut u8>) -> Option<*mut u8> {
    let ptr = s.map(|s| s.as_mut_ptr()).unwrap_or((*state)?);
    // 使用 strspn 跳过分隔符，strcspn 定位 token 结束
    None
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  strspn_impl     // 依赖1: 跳过前导分隔符
  strcspn_impl    // 依赖2: 定位分隔符

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strtok_r(s: *mut core::ffi::c_char, sep: *const core::ffi::c_char, p: *mut *mut core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strtok_r_impl(state: &mut Option<*mut u8>, sep: &[u8]) -> Option<*mut u8>;