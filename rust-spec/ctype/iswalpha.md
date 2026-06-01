# iswalpha 函数规约 (Rust)

## 复杂度分级: Level 3

---

## 类型定义

```rust
// wint_t: 对应 C 的 typedef unsigned wint_t (来自 <bits/alltypes.h>)
// 在 musl 中 wint_t 即 unsigned int, Rust 中映射为 core::ffi::c_uint (u32)
pub type wint_t = core::ffi::c_uint;

// locale_t: 对应 C 的 struct __locale_struct * (来自 <bits/alltypes.h>)
// 不透明结构体指针, 具体定义见 src/internal/locale_impl.h
#[repr(C)]
pub struct __locale_struct { _opaque: [u8; 1] }
pub type locale_t = *mut __locale_struct;

// WEOF: 对应 C 的 #define WEOF 0xffffffffU (来自 <wctype.h>)
pub const WEOF: wint_t = 0xffffffff_u32;
```

---

## 函数接口

```rust
use core::ffi::c_int;

/// 静态位图查找表, 由 alpha.h 编译时生成
/// [Visibility]: Internal — 模块私有静态只读数据, 不对外导出
/// 对应 C 中 static const unsigned char table[] = { #include "alpha.h" };
/// Rust 实现: const TABLE: &[u8] 或 static TABLE: [u8; N] (编译期常量化)
static TABLE: [u8; TABLE_SIZE] = {
    // 从 alpha.h 转换生成, 编译期确定
    // 注: alpha.h 是 bit 位图而非字节数组, Rust 中保持相同的 bit 编码格式
    include!("alpha_data.rs")  // 或使用 include_bytes! 后 cast
};

/// C 标准 iswalpha — 判断宽字符是否为 Unicode 字母
/// [Visibility]: External — POSIX.1-2001 标准函数, <wctype.h> 声明, ABI 兼容
pub extern "C" fn iswalpha(wc: wint_t) -> c_int;

/// iswalpha_l_impl — iswalpha_l 的内部实现体 (C 中对应 __iswalpha_l)
/// [Visibility]: Internal — musl 内部符号, 不直接对外导出
/// Rust 无 weak_alias 机制, 更名为 iswalpha_l_impl 并作为模块内部函数
pub(crate) fn iswalpha_l_impl(wc: wint_t, l: locale_t) -> c_int;

/// iswalpha_l — locale-aware 宽字符字母判断
/// [Visibility]: External — POSIX.1-2008 标准函数, extern "C" 导出, ABI 兼容
/// 注: musl 不区分 locale, l 参数被忽略, 行为与 iswalpha 完全一致
pub extern "C" fn iswalpha_l(wc: wint_t, l: locale_t) -> c_int;
```

### 符号可见性

| 符号 | 可见性 | 导出方式 | 说明 |
|---|---|---|---|
| `iswalpha` | External | `#[no_mangle] pub extern "C"` | POSIX.1-2001 标准, `<wctype.h>` 声明 |
| `iswalpha_l` | External | `#[no_mangle] pub extern "C"` | POSIX.1-2008 标准, `<wctype.h>` 声明 |
| `iswalpha_l_impl` | Internal | `pub(crate) fn` (模块私有) | 共享实现体, C 中对应 `__iswalpha_l` |
| `TABLE` | Internal | `static` (模块私有) | 位图查找表, 从 `alpha.h` 转换生成 |

## iswalpha (对外导出)

**[Visibility]: External** — POSIX.1-2001 标准函数，`<wctype.h>` 声明。

### 前置/后置条件

**[Pre-condition]:**
- `wc`: 类型为 `wint_t` (即 `c_uint`)，任意宽字符值（含 `WEOF` = `0xffffffff_u32`）。

**[Post-condition]:**
- Case 1: `wc` 是 Unicode 字母字符
  - `wc < 0x20000_u32` 且二级位图查找命中：返回 `1`
  - `wc` 在 `[0x20000_u32, 0x2fffe_u32)` 范围内（CJK Extension B 区段）：返回 `1`
- Case 2: `wc` 不是字母字符
  - `wc >= 0x2fffe_u32`：返回 `0`
  - `wc < 0x20000_u32` 但位图查找未命中：返回 `0`
- Case 3: `wc == WEOF` (即 `0xffffffff_u32`)
  - `0xffffffff_u32 >= 0x20000_u32`，但 `>= 0x2fffe_u32`，进入 Phase 3 返回 `0`。

### 不变量

**[Invariant]:**
- `TABLE` 静态数组为编译期只读常量，程序生命周期内不可变。Rust 中通过 `static` + 不可变绑定保证。
- 函数为纯函数，无副作用，无内部可变状态，完全线程安全。
- 不依赖 locale 设置。

### 意图

判断宽字符是否为 Unicode 字母字符。使用二级位图查找表覆盖 BMP 及 Supplementary Multilingual Plane（到 U+1FFFF）的所有码点，对 CJK Extension B 范围（U+20000-U+2FFFD）使用硬编码返回 `true`。位图表来自 Unicode 字符数据库编译生成的 `alpha.h` 头文件。

Rust 实现策略：
- 将 `alpha.h` 的 bit 位图数据转换为 Rust 兼容的 `[u8; N]` 静态数组
- 或使用 `include_bytes!("alpha_data.bin")` 将预处理的二进制位图直接嵌入
- 利用 Rust 的边界检查和 `get_unchecked` 等方法在保证安全的前提下达到与 C 等价的零成本位图访问

### 系统算法

```rust
fn iswalpha(wc: wint_t) -> c_int {
    // Phase 1（快速路径 - BMP 及 SMP 到 U+1FFFF）:
    //   wc < 0x20000 时，使用二级位图查找:
    //     l1_idx = wc >> 8                        // 高 8 位 (一级索引)
    //     l1_val = TABLE[l1_idx as usize]         // 获取二级表偏移量
    //     l2_idx = l1_val * 32 + ((wc & 255) >> 3) // 二级索引 (字节偏移)
    //     bit_mask = 1u8 << (wc & 7)              // 位掩码
    //     命中判定: TABLE[l2_idx as usize] & bit_mask != 0
    //
    // Phase 2（CJK Extension B - U+20000 到 U+2FFFD）:
    //   0x20000 <= wc < 0x2fffe 时直接返回 1
    //
    // Phase 3（越界）:
    //   wc >= 0x2fffe 返回 0
    //
    if wc < 0x20000_u32 {
        // 一级索引: wc >> 8
        let l1_idx = (wc >> 8) as usize;
        // 二级表偏移 (包含一级索引表+二级位图)
        // 注: C 版本中 table[wc>>8] 读取一级, table[table[wc>>8]*32 + ((wc&255)>>3)] 读取二级
        let l2_base = TABLE[l1_idx] as usize * 32;
        let l2_idx = l2_base + ((wc as usize & 255) >> 3);
        let bit_mask = 1u8 << (wc & 7);
        (TABLE[l2_idx] & bit_mask != 0) as c_int
    } else if wc < 0x2fffe_u32 {
        1
    } else {
        0
    }
}

fn iswalpha_l_impl(wc: wint_t, _l: locale_t) -> c_int {
    iswalpha(wc)
}
```

时间复杂度 O(1)。Phase 1 需要 2 次内存读取（一级 + 二级索引），均在同一个 `TABLE` 静态数组中，具有良好的缓存局部性。Phase 2/3 无内存访问。

---

## 位图数据转换说明

C 源文件 `alpha.h` 包含 `unsigned char` 类型的数组字面量，格式为:
```c
// alpha.h 片段示例
 12,  3,255,  1, 64, ...
```

Rust 实现需将此数据转换为 Rust 兼容格式。推荐方案：

1. **构建脚本生成 (build.rs)**: 在 `build.rs` 中解析 `alpha.h` 原始数据，生成 `alpha_data.rs` 文件，内容为 `const ALPHA_DATA: [u8; N] = [12, 3, 255, 1, 64, ...];`，然后在源文件中 `include!("alpha_data.rs")`。

2. **二进制嵌入**: 预处理 `alpha.h` 为纯二进制文件 `alpha_data.bin`，在 Rust 中使用:
   ```rust
   static TABLE: &[u8] = include_bytes!("alpha_data.bin");
   ```
   此方案最简洁且无需构建脚本。

两种方案均兼容 `#![no_std]` 环境（不依赖 alloc 或 std），且保证数据在编译期嵌入只读段 (.rodata)。

---

## 跨文件依赖摘要

| 依赖符号 | 来源 | 可见性 | 说明 |
|---|---|---|---|
| `wint_t` | 本模块定义 (`c_uint` 别名) | 系统类型 | `c_uint` 的 pub type 别名 |
| `locale_t` | 本模块引用 (`*mut __locale_struct`) | 系统类型 | `*mut __locale_struct` 的 pub type 别名 |
| `WEOF` | 本模块定义 | 模块常量 | 值为 `0xffffffff_u32` |
| `alpha.h` (TABLE) | 编译时数据文件 | Internal | Unicode 字母属性位图, 编译时嵌入 |

---

## 依赖汇总

```
[RELY]
Predefined Types:
  c_int                     // core::ffi::c_int, 对应 C 的 int (函数返回类型)
  c_uint                    // core::ffi::c_uint, 对应 C 的 unsigned int
  wint_t                    // pub type wint_t = c_uint;
  __locale_struct           // 不透明结构体, 见 src/internal/locale_impl.h
                            // Rust: #[repr(C)] struct __locale_struct { _opaque: [u8; 1] }
  locale_t                  // pub type locale_t = *mut __locale_struct;
  WEOF                      // pub const WEOF: wint_t = 0xffffffff_u32;

Predefined Data (Internal, 编译时嵌入):
  alpha.h 位图数据           // Unicode 字母属性二级位图表
                            // Rust 中通过 build.rs 生成或 include_bytes! 嵌入
                            // 转换为 static TABLE: [u8; N] 或 static TABLE: &[u8]

Predefined Functions:
  (无外部函数依赖 — iswalpha 为叶子函数, 仅依赖内部静态位图表, 不调用其他 is*/isw* 族函数)

[GUARANTEE]
Exported Interface (extern "C", ABI 兼容):
  pub extern "C" fn iswalpha(wc: wint_t) -> c_int;
                              // 本模块保证对外提供的 iswalpha 接口签名与行为
  pub extern "C" fn iswalpha_l(wc: wint_t, l: locale_t) -> c_int;
                              // 本模块保证对外提供的 iswalpha_l 接口签名与行为
                              // 注: l 参数被忽略, 行为等效于 iswalpha(wc)

Internal Interface (不对外导出):
  pub(crate) fn iswalpha_l_impl(wc: wint_t, l: locale_t) -> c_int;
                              // 内部实现函数, 作为 iswalpha_l 的委托目标
                              // C 中对应 __iswalpha_l, Rust 中重命名为 iswalpha_l_impl
  static TABLE: [u8; TABLE_SIZE]  // 位图查找表, 编译期常量
                              // 从 alpha.h 转换生成, 生命周期内不可变
```