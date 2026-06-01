# ispunct 函数规约 (Rust)

## 复杂度分级: Level 1

---

## 函数接口

```rust
use core::ffi::c_int;

// locale_t 类型: 不透明指针, 对应 C 的 struct __locale_struct *
// 具体定义见 src/internal/locale_impl.h, Rust 中作为 extern type 或 repr(C) 不透明结构体声明
// 注: musl 的 _l 后缀函数忽略 locale 参数, 故内部无需解引用该指针

/// C 标准 ispunct — 判断字符是否为标点符号
/// [Visibility]: External — POSIX.1-2001 标准函数, <ctype.h> 声明, ABI 兼容
pub extern "C" fn ispunct(c: c_int) -> c_int;

/// __ispunct_l — ispunct_l 的内部实现体 (musl 中通过 weak_alias 将 ispunct_l 链接至此)
/// [Visibility]: Internal — musl 内部符号, 不直接对外导出
/// 注: Rust 无 weak_alias 机制, 本函数可作为 ispunct_l 的回退实现体,
///     或由 ispunct_l 直接内联调用共享的内部实现函数
fn ispunct_l_impl(c: c_int, l: *mut __locale_struct) -> c_int;

/// ispunct_l — locale-aware 标点符号判断
/// [Visibility]: External — POSIX.1-2008 标准函数, extern "C" 导出, ABI 兼容
/// 注: musl 不区分 locale, l 参数被忽略, 行为与 ispunct 完全一致
pub extern "C" fn ispunct_l(c: c_int, l: *mut __locale_struct) -> c_int;
```

### 前置/后置条件

**[Pre-condition]:**
- `c`: 类型为 `c_int` (对应 C 的 `int`)，值必须可表示为 `unsigned char` 或等于 `EOF` (`-1`)。

**[Post-condition]:**
- Case 1: `c` 是标点符号（可打印图形字符但非字母非数字）
  - 即 `isgraph(c) && !isalnum(c)` 为 `true` 时返回非零值 (musl 中为 `1`)。
- Case 2: 其他字符或 `EOF`
  - 返回 `0`。

**[Post-condition] (ispunct_l / ispunct_l_impl):**
- 完全等效于 `ispunct(c)` 的返回值。
- `l` 参数（`*mut __locale_struct`）被忽略，所有 locale 下行为一致。

### 不变量

**[Invariant]:**
- 纯函数，无副作用，无内部可变状态，完全线程安全。
- 不依赖 locale 设置。`_l` 后缀版本的 locale 参数预留但未使用（musl 不支持 per-locale 字符分类）。

### 意图

判断字符是否为标点符号。标点符号定义为可打印图形字符中排除字母和数字的部分。musl 通过组合 `isgraph` 和 `isalnum` 实现，避免独立维护标点符号位图表。

### 系统算法

```
fn ispunct(c: c_int) -> c_int {
    // 直接组合 isgraph 和 isalnum:
    //   isgraph: (unsigned)c - 0x21 < 0x5e  → 检查是否为可打印图形字符(0x21-0x7E)
    //   isalnum: isalpha(c) || isdigit(c)    → 检查是否为字母或数字
    // 标点 = 图形字符且非字母非数字
    if isgraph(c) != 0 && isalnum(c) == 0 { 1 } else { 0 }
}

fn ispunct_l_impl(c: c_int, l: *mut __locale_struct) -> c_int {
    // l 参数被忽略, 委托给 ispunct
    ispunct(c)
}
```

时间复杂度 O(1)。最坏情况：1 次 `isgraph` 检查 + 1 次 `isalnum` 检查（即 1 次无符号区间比较 + 1 次字母表查找或数字区间比较）。

---

## 依赖汇总

```
[RELY]
Predefined Types:
  c_int                     // core::ffi::c_int, 对应 C 的 int
  __locale_struct           // 不透明结构体, 见 src/internal/locale_impl.h
                            // Rust 中声明为 #[repr(C)] struct __locale_struct { _opaque: [u8; 1] }
  locale_t                  // *mut __locale_struct, 对应 C 的 locale_t (struct __locale_struct *)
  EOF                       // 宏常量, 值为 -1 (即 c_int 下的 -1)

Predefined Functions (External, extern "C" 导出):
  isgraph(c: c_int) -> c_int  // 外部依赖: src/ctype/isgraph.c, 判断是否为可打印图形字符
  isalnum(c: c_int) -> c_int  // 外部依赖: src/ctype/isalnum.c, 判断是否为字母或数字

[GUARANTEE]
Exported Interface (extern "C", ABI 兼容):
  pub extern "C" fn ispunct(c: c_int) -> c_int;
                              // 本模块保证对外提供的 ispunct 接口签名与行为
  pub extern "C" fn ispunct_l(c: c_int, l: *mut __locale_struct) -> c_int;
                              // 本模块保证对外提供的 ispunct_l 接口签名与行为
                              // 注: l 参数被忽略, 行为等效于 ispunct(c)

Internal Interface (不对外导出):
  fn ispunct_l_impl(c: c_int, l: *mut __locale_struct) -> c_int;
                              // 内部实现函数, 作为 ispunct_l 的核心逻辑体
                              // Rust 无 weak_alias 机制, 可由 ispunct_l 直接调用或内联
```