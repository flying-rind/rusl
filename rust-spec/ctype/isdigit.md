# isdigit 函数归约 (Rust)

## 复杂度分级: Level 1

---

## 函数接口

```rust
use core::ffi::c_int;

// locale_t: 不透明指针类型, ABI 等价于 C locale_t (struct __locale_struct*)
// 定义于 crate::locale 模块

// [Visibility]: External
extern "C" fn isdigit(c: c_int) -> c_int;

// [Visibility]: Internal - 不对外导出
// 内部实现, isdigit_l 通过 weak_alias 符号别名指向此函数
fn __isdigit_l(c: c_int, l: locale_t) -> c_int;

// [Visibility]: External
// POSIX.1-2008 locale-aware 变体, weak_alias -> __isdigit_l
extern "C" fn isdigit_l(c: c_int, locale: locale_t) -> c_int;
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `c_int`（ABI 等价于 C `int`），值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是十进制数字（`'0'`-`'9'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为十进制数字字符。使用经典的无符号区间检查技巧。

### 系统算法

```rust
// 无符号减法区间检查, 编译为无分支代码
(c as u32).wrapping_sub(b'0' as u32) < 10
// 通过 u32 减法将 '0'-'9' 映射到 0-9, 同时将 EOF（-1）映射为大值（>= 10）
// 时间复杂度 O(1), 无分支
```

---

/* Rely */
[RELY]
类型依赖:
  locale_t;                       // 依赖1: locale 模块定义的不透明指针类型, ABI 等价于 C locale_t

[GUARANTEE]
Exported Interface:
  extern "C" fn isdigit(c: c_int) -> c_int;
    // [Visibility]: External
    // 判断 c 是否为十进制数字字符（'0'-'9'）
    // 返回值: 数字字符返回非零值, 否则返回 0
  extern "C" fn isdigit_l(c: c_int, locale: locale_t) -> c_int;
    // [Visibility]: External
    // isdigit 的 locale-aware 变体
    // 在 C/POSIX locale 下与 isdigit 行为相同, locale 参数被忽略
    // 本模块保证对外提供该接口签名, ABI 兼容 C isdigit_l