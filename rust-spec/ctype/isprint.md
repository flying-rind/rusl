# isprint 函数归约 (Rust)

## 复杂度分级: Level 1

---

## 函数接口

```rust
use core::ffi::c_int;

// locale_t: 不透明指针类型, ABI 等价于 C locale_t (struct __locale_struct*)
// 定义于 crate::locale 模块

// [Visibility]: External
extern "C" fn isprint(c: c_int) -> c_int;

// [Visibility]: Internal - 不对外导出
// 内部实现, isprint_l 通过 weak_alias 符号别名指向此函数
fn __isprint_l(c: c_int, l: locale_t) -> c_int;

// [Visibility]: External
// POSIX.1-2008 locale-aware 变体, weak_alias -> __isprint_l
extern "C" fn isprint_l(c: c_int, locale: locale_t) -> c_int;
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `c_int`（ABI 等价于 C `int`），值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是可打印字符（0x20-0x7E，包含空格）
  - 返回非零值。
- Case 2: 其他字符（控制字符、DEL 等）或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为可打印字符（含空格，即 `isgraph(c) || c == ' '`）。

### 系统算法

```rust
// 无符号减法区间检查, 编译为无分支代码
(c as u32).wrapping_sub(0x20) < 0x5f
// 通过 u32 区间检查: 0x20 到 0x7E（含）映射到 0 到 0x5E
// 时间复杂度 O(1), 无分支
```

---

/* Rely */
[RELY]
类型依赖:
  locale_t;                       // 依赖1: locale 模块定义的不透明指针类型, ABI 等价于 C locale_t

[GUARANTEE]
Exported Interface:
  extern "C" fn isprint(c: c_int) -> c_int;
    // [Visibility]: External
    // 判断 c 是否为可打印字符（0x20-0x7E, 含空格）
    // 返回值: 可打印字符返回非零值, 否则返回 0
  extern "C" fn isprint_l(c: c_int, locale: locale_t) -> c_int;
    // [Visibility]: External
    // isprint 的 locale-aware 变体
    // 在 C/POSIX locale 下与 isprint 行为相同, locale 参数被忽略
    // 本模块保证对外提供该接口签名, ABI 兼容 C isprint_l