# strlen — Rust 接口归约

## 原始 C 接口
```c
size_t strlen(const char *s);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn strlen(s: *const core::ffi::c_char) -> usize;
```

---

## 意图
计算以 null 结尾的 UTF-8/C 字符串的长度（字节数，不含 null 终止符）。对外导出 C ABI 兼容的 `strlen` 符号供链接器使用。

## 前置条件
- `s` 为非空指针（`!s.is_null()`）
- `s` 指向以 `\0` 终止的有效字节序列
- 若作为 `&CStr` 使用则要求字节序列为有效 UTF-8 子集（ASCII 或按平台约定）

## 后置条件
- 返回值为 `\0` 在 `s` 指向序列中首次出现的字节偏移量
- 返回值类型为 `usize`，在 64 位平台上与 C 的 `size_t` 兼容

## 不变量
- 搜索指针从 `s` 起单调递增，直至找到 `\0`
- 字级别加速时 `HASZERO`（或 Rust 等效技巧）正确检测含零字节的字

## 算法
由于 Rust 核心库 `core::ffi::CStr` 已提供 `to_bytes().len()` 实现，内部可直接复用：

```rust
use core::ffi::CStr;

// 方法一：直接复用 CStr
pub fn strlen_impl(s: &CStr) -> usize {
    s.to_bytes().len()
}

// 方法二：若需要手写实现，复刻 C 原算法的字对齐加速逻辑
// 使用 usize::from_le_bytes / to_ne_bytes 替代 C 的 HASZERO 宏
```

对 C 调用者：
1. `extern "C" fn strlen(s: *const c_char) -> usize` 接收裸指针
2. 内部构造 `unsafe { CStr::from_ptr(s) }`，调用 `count_bytes()`
3. 返回 `usize` 即字节数

若出于性能或 no_std 环境要求（无 `alloc`），可以直接复刻 C 算法的 Rust 版本：使用 `ptr::read_unaligned` 在字级读取并检测零字节。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
pub(crate) fn str_len(s: &core::ffi::CStr) -> usize {
    s.count_bytes()
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr               // 依赖1: Rust 核心库提供的 C 字符串视图
  core::ptr::read_unaligned     // 依赖2: 非对齐读取（若手写字级别加速）
Predefined Macros/Traits:
  (none)                         // 所有辅助逻辑均可在函数内定义

[GUARANTEE]
Exported Interface:
  extern "C" fn strlen(s: *const core::ffi::c_char) -> usize;
                                 // 本模块保证对外提供与 C ABI 兼容的 strlen 符号
Internal Interface:
  pub(crate) fn str_len(s: &core::ffi::CStr) -> usize;
                                 // 安全包装，供 crate 内部使用