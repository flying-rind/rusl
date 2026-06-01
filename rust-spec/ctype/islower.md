# islower 函数归约 (Rust)

## 复杂度分级: Level 1

---

## 函数接口

```rust
use core::ffi::c_int;

// locale_t: 不透明指针类型, ABI 等价于 C locale_t (struct __locale_struct*)
// 定义于 crate::locale 模块

// [Visibility]: External
extern "C" fn islower(c: c_int) -> c_int;

// [Visibility]: Internal - 不对外导出
// 内部实现, islower_l 通过 weak_alias 符号别名指向此函数
fn __islower_l(c: c_int, l: locale_t) -> c_int;

// [Visibility]: External
// POSIX.1-2008 locale-aware 变体, weak_alias -> __islower_l
extern "C" fn islower_l(c: c_int, locale: locale_t) -> c_int;
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `c_int`（ABI 等价于 C `int`），值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是小写字母（`'a'`-`'z'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为小写英文字母。使用无符号区间检查避免分支。

### 系统算法

```rust
// 无符号减法区间检查, 编译为无分支代码
(c as u32).wrapping_sub(b'a' as u32) < 26
// 通过 u32 减法将 'a'-'z' 映射到 0-25, EOF 映射为大值
// 时间复杂度 O(1), 无分支
```

---

/* Rely */
[RELY]
类型依赖:
  locale_t;                       // 依赖1: locale 模块定义的不透明指针类型, ABI 等价于 C locale_t

[GUARANTEE]
Exported Interface:
  extern "C" fn islower(c: c_int) -> c_int;
    // [Visibility]: External
    // 判断 c 是否为小写英文字母（'a'-'z'）
    // 返回值: 小写字母返回非零值, 否则返回 0
  extern "C" fn islower_l(c: c_int, locale: locale_t) -> c_int;
    // [Visibility]: External
    // islower 的 locale-aware 变体
    // 在 C/POSIX locale 下与 islower 行为相同, locale 参数被忽略
    // 本模块保证对外提供该接口签名, ABI 兼容 C islower_l