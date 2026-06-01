# isspace 函数规约 (Rust)

## 复杂度分级: Level 1

---

## 函数接口

```rust
use core::ffi::c_int;

// locale_t 类型: 不透明指针, 对应 C 的 struct __locale_struct *
// 具体定义见 src/internal/locale_impl.h, Rust 中作为 extern type 或 repr(C) 不透明结构体声明
// 注: musl 的 _l 后缀函数忽略 locale 参数, 故内部无需解引用该指针

/// C 标准 isspace — 判断字符是否为空白字符
/// [Visibility]: External — POSIX.1-2001 标准函数, <ctype.h> 声明, ABI 兼容
pub extern "C" fn isspace(c: c_int) -> c_int;

/// __isspace_l — isspace_l 的内部实现体 (musl 中通过 weak_alias 将 isspace_l 链接至此)
/// [Visibility]: Internal — musl 内部符号, 不直接对外导出
/// 注: Rust 无 weak_alias 机制, 本函数可作为 isspace_l 的回退实现体,
///     或由 isspace_l 直接内联调用共享的内部实现函数
fn isspace_l_impl(c: c_int, l: *mut __locale_struct) -> c_int;

/// isspace_l — locale-aware 空白字符判断
/// [Visibility]: External — POSIX.1-2008 标准函数, extern "C" 导出, ABI 兼容
/// 注: musl 不区分 locale, l 参数被忽略, 行为与 isspace 完全一致
pub extern "C" fn isspace_l(c: c_int, l: *mut __locale_struct) -> c_int;
```

### 符号可见性

按 Rust 项目约定，符号分为两类：

| 符号 | 可见性 | 导出方式 | 说明 |
|---|---|---|---|
| `isspace` | External | `#[no_mangle] pub extern "C"` | POSIX.1-2001 标准, `<ctype.h>` 声明 |
| `isspace_l` | External | `#[no_mangle] pub extern "C"` | POSIX.1-2008 标准, `<ctype.h>` 声明 |
| `isspace_l_impl` | Internal | `pub(crate) fn` (模块私有) | 共享实现体, C 中对应 `__isspace_l` |

注: Rust 无 `weak_alias` 机制，将 `__isspace_l` 更名为 `isspace_l_impl` 并作为模块内部函数。`isspace_l` 直接定义为独立的 `extern "C"` 函数，内部委托给 `isspace_l_impl`。对外部调用者而言，`isspace_l` 的行为与 C 的 `weak_alias(__isspace_l, isspace_l)` 等价。

### 前置/后置条件

**[Pre-condition]:**
- `c`: 类型为 `c_int` (对应 C 的 `int`)，值必须可表示为 `unsigned char` 或等于 `EOF` (`-1`)。

**[Post-condition]:**
- Case 1: `c` 是 C 标准空白字符（`' '`、`'\t'`、`'\n'`、`'\v'`、`'\f'`、`'\r'`）
  - 返回非零值 (musl 中为 `1`)。
- Case 2: 其他字符或 `EOF`
  - 返回 `0`。

**[Post-condition] (isspace_l / isspace_l_impl):**
- 完全等效于 `isspace(c)` 的返回值。
- `l` 参数（`*mut __locale_struct`）被忽略，所有 locale 下行为一致。

### 不变量

**[Invariant]:**
- 纯函数，无副作用，无内部可变状态，完全线程安全。
- 不依赖 locale 设置。`_l` 后缀版本的 locale 参数预留但未使用（musl 不支持 per-locale 字符分类）。

### 意图

判断字符是否为 C 标准空白字符。使用紧凑的无符号区间技巧：`(c as c_uint).wrapping_sub('\t' as c_uint) < 5` 覆盖 `'\t'`(9) 到 `'\r'`(13)（含）五个连续空白字符，再单独检查空格 `' '`(32)。该技巧将 6 个空白字符的检查压缩为 1 次无符号比较 + 1 次相等比较，无分支预测开销。

### 系统算法

```rust
fn isspace(c: c_int) -> c_int {
    // Step 1: 检查空格 (0x20)
    //   c == ' '
    // Step 2: 检查 '\t'(9) ~ '\r'(13) 的连续空白区间
    //   (c as c_uint).wrapping_sub('\t' as c_uint) < 5
    //   区间映射:
    //     '\t'(9)  -> 0
    //     '\n'(10) -> 1
    //     '\v'(11) -> 2
    //     '\f'(12) -> 3
    //     '\r'(13) -> 4
    //   EOF(-1) 在无符号转换后为 0xFFFFFFFFu32, wrapping_sub(9) 溢出为大值 >> 5
    if c == ' ' as c_int || (c as c_uint).wrapping_sub('\t' as c_uint) < 5 {
        1
    } else {
        0
    }
}

fn isspace_l_impl(c: c_int, _l: *mut __locale_struct) -> c_int {
    isspace(c)
}
```

时间复杂度 O(1)，至多 2 次无符号比较，无分支（可通过 cmov 实现）。

---

## 依赖汇总

```
[RELY]
Predefined Types:
  c_int                     // core::ffi::c_int, 对应 C 的 int
  c_uint                    // core::ffi::c_uint, 对应 C 的 unsigned int, 用于无符号区间技巧
  __locale_struct           // 不透明结构体, 见 src/internal/locale_impl.h
                            // Rust 中声明为 #[repr(C)] struct __locale_struct { _opaque: [u8; 1] }
  locale_t                  // *mut __locale_struct, 对应 C 的 locale_t (struct __locale_struct *)
  EOF                       // 宏常量, 值为 -1_i32

Predefined Functions:
  (无外部函数依赖 — isspace 为叶子函数, 不调用其他 is*/isw* 族函数)

[GUARANTEE]
Exported Interface (extern "C", ABI 兼容):
  pub extern "C" fn isspace(c: c_int) -> c_int;
                              // 本模块保证对外提供的 isspace 接口签名与行为
  pub extern "C" fn isspace_l(c: c_int, l: *mut __locale_struct) -> c_int;
                              // 本模块保证对外提供的 isspace_l 接口签名与行为
                              // 注: l 参数被忽略, 行为等效于 isspace(c)

Internal Interface (不对外导出):
  fn isspace_l_impl(c: c_int, l: *mut __locale_struct) -> c_int;
                              // 内部实现函数, 作为 isspace_l 的核心逻辑体
                              // C 中对应 __isspace_l, Rust 中作为 isspace_l 的委托目标
```