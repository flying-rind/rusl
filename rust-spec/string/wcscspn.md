# wcscspn — Rust 接口归约

## 原始 C 接口
```c
size_t wcscspn(const wchar_t *s, const wchar_t *c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcscspn(s: *const u32, c: *const u32) -> usize;
```

---

## 意图
计算 s 的起始段长度，该段中不包含宽字符串 c 中的任何宽字符。

## 前置条件
- `s` 非空、`c` 非空
- s 和 c 以 L'\0' 结尾

## 后置条件
- 返回 s 中首个匹配 c 中任意字符的位置索引
- s 和 c 内容不变

## 不变量
- s 指针单调递增

## 算法
因宽字符集太大无法使用位图，采用朴素 O(|s|*|c|) 搜索：

```rust
pub fn wcscspn_impl(s: &[u32], reject: &[u32]) -> usize {
    let reject_set: Vec<_> = reject.iter().take_while(|&&c| c != 0).copied().collect();
    s.iter().position(|&ch| ch == 0 || reject_set.contains(&ch)).unwrap_or(s.len())
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::position       // 依赖1: 位置查找
  core::iter::Iterator::take_while     // 依赖2: 截取非 null 元素

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcscspn(s: *const u32, c: *const u32) -> usize;
Internal Interface:
  pub(crate) fn wcscspn_impl(s: &[u32], reject: &[u32]) -> usize;