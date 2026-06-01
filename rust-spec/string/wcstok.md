# wcstok — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcstok(wchar_t *restrict s, const wchar_t *restrict sep, wchar_t **restrict p);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcstok(s: *mut u32, sep: *const u32, p: *mut *mut u32) -> *mut u32;
```

---

## 意图
从宽字符串 s 中提取下一个 token，使用调用者提供的指针 *p 维护状态（可重入版本）。

## 前置条件
- `p` 非空
- 首次调用 `s` 非空，后续可传 null
- `sep` 非空，以 L'\0' 结尾

## 后置条件
- 若无更多 token，返回 null，*p 设为 null
- 若有 token，返回 token 起始指针，分隔符位置被 L'\0' 替换
- *p 更新为下一搜索位置

## 不变量
- *p 为 null 或指向下一搜索起始位置

## 算法
使用 wcsspn 和 wcscspn 提取 token：

```rust
pub fn wcstok_impl(state: &mut Option<*mut u32>, sep: &[u32]) -> Option<*mut u32> {
    let ptr = (*state)?;
    // 使用 wcsspn 跳过分隔符，wcscspn 定位分隔符
    None
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  wcsspn_impl   // 依赖1: 跳过前导分隔符
  wcscspn_impl  // 依赖2: 定位分隔符

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcstok(s: *mut u32, sep: *const u32, p: *mut *mut u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcstok_impl(state: &mut Option<*mut u32>, sep: &[u32]) -> Option<*mut u32>;