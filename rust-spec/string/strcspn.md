# strcspn — Rust 接口归约

## 原始 C 接口
```c
size_t strcspn(const char *s, const char *c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strcspn(s: *const core::ffi::c_char, c: *const core::ffi::c_char) -> usize;
```

---

## 意图
计算 s 的起始段长度，该段中不包含字符串 c 中的任何字符。

## 前置条件
- `s` 非空、`c` 非空
- s 和 c 以 null 结尾

## 后置条件
- 返回 s 中首个匹配 c 中任意字符的位置索引（或 strlen(s) 若未找到）
- s 和 c 内容不变

## 不变量
- s 指针单调递增

## 算法
构建位图快速检测字符：

```rust
pub fn strcspn_impl(s: &core::ffi::CStr, reject: &core::ffi::CStr) -> usize {
    let mut bitset = [0u8; 32];
    for &b in reject.to_bytes() { bitset[(b as usize) >> 3] |= 1 << (b & 7); }
    s.to_bytes().iter()
        .position(|&b| bitset[(b as usize) >> 3] & (1 << (b & 7)) != 0)
        .unwrap_or(s.count_bytes())
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes      // 依赖1: 字节切片
  core::ffi::CStr::count_bytes   // 依赖2: 字符串长度
  core::iter::Iterator::position // 依赖3: 位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strcspn(s: *const core::ffi::c_char, c: *const core::ffi::c_char) -> usize;
Internal Interface:
  pub(crate) fn strcspn_impl(s: &core::ffi::CStr, reject: &core::ffi::CStr) -> usize;