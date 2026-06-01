# strerror 函数规约

## 复杂度分级: Level 1

> musl libc `strerror`/`strerror_l` 实现。将 errno 错误码映射为人可读的错误描述字符串，支持 locale 本地化。

---

## 函数接口

```rust
use core::ffi::{c_int, c_char, c_void};

// locale_t 为 opaque 指针类型，在 Rust 侧用 *mut c_void 表示以保持 ABI 兼容
type locale_t = *mut c_void;

extern "C" fn strerror(e: c_int) -> *mut c_char;

// weak_alias: strerror_l 是 __strerror_l 的弱别名，共享同一实现
extern "C" fn strerror_l(e: c_int, loc: locale_t) -> *mut c_char;
```

[Visibility]:
- `strerror`: Public -- ISO C 标准函数 (`<string.h>`)，`#[no_mangle]` 导出
- `strerror_l`: Public -- POSIX.1-2008 扩展，通过弱别名 `#[no_mangle]` 与 `__strerror_l` 共享实现体

---

### 前置/后置条件

**[Pre-condition]:**

- `strerror(e)`: `e` 类型为 `c_int`，为 errno 错误码值（通常 0..133 范围内，但可接受任意 `c_int` 值）。
- `strerror_l(e, loc)`: `e` 同上；`loc` 为有效的 `locale_t` 句柄。

**[Post-condition]:**

- *Case 1*: `0 <= e < len(errmsg_table)`（已知错误码）
  - 若 `e == 0`，返回指向 `"No error information"` 的指针
  - 否则返回指向该错误码对应描述字符串的指针
  - 返回值经 `LCTRANS` / `__lctrans` 进行 locale 消息翻译

- *Case 2*: `e < 0` 或 `e >= len(errmsg_table)`（超出范围的未知错误码）
  - 将 `e` 重置为 0，返回指向 `"No error information"` 的指针

- *Case 3*: MIPS EDQUOT 兼容（编译时通过 `#[cfg]` 条件处理）
  - 若 `e == EDQUOT`（内部重映射值 109），将其映射为 0
  - 若 `e == EDQUOT_ORIG`（MIPS 原始值 1133），将其映射为内部 EDQUOT 值

**[Error Behavior]:**
- 本函数不设置 errno。
- 对于任何输入的 `e` 值，始终返回有效的 NUL 结尾字符串指针。

---

### 不变量

**[Invariant]:**
- 返回值始终指向有效的 NUL 结尾静态字符串，调用者不应释放该指针。
- 返回的指针指向静态只读存储（`&'static CStr` 或等同的 `*const c_char`）。
- 函数不修改任何共享状态，线程安全（只读访问静态表 + LCTRANS 翻译）。
- `strerror_l` 与 `__strerror_l` 行为完全一致（共享同一函数体）。
- 对于同一错误码 `e` 和同一 locale `loc`，每次调用返回相同的结果（幂等性）。

---

### 意图

将系统 errno 错误码转换为人类可读的错误描述字符串。`strerror` 使用当前线程的 locale 进行翻译，`strerror_l` 使用指定的 locale 进行翻译。

Rust 侧实现：
- 内部使用静态数组 `ERROR_MESSAGES: &[&CStr]` 或等价的字节切片数组，按错误码直接索引。未知/超出范围的错误码回退到索引 0（`"No error information"`）。
- `strerror` 内部调用 `__strerror_l(e, CURRENT_LOCALE)`。
- MIPS EDQUOT 特殊处理通过 `#[cfg(target_arch = "mips")]`（或 `#[cfg(target_arch = "mips64")]`）条件编译实现。
- `LCTRANS` 是 locale 消息翻译基础设施，由 locale 模块提供（`__lctrans` 函数或等价的内部接口）。

---

### 系统算法

```
strerror(e):
  return __strerror_l(e, CURRENT_LOCALE)

__strerror_l(e, loc):
  // MIPS EDQUOT 兼容（仅在 MIPS 架构上编译）
  #[cfg(mips_edquot)]:
    if e == EDQUOT:       e = 0
    elif e == EDQUOT_ORIG: e = EDQUOT

  // 边界检查
  if e < 0 || e >= ERROR_MESSAGES.len():
    e = 0

  // 查表 + locale 翻译
  msg = ERROR_MESSAGES[e]  // 获取静态 C 字符串指针
  return LCTRANS(msg, loc)  // locale 消息翻译
```

时间复杂度 O(1)（数组直接索引 + locale 翻译查找）。

---

## 依赖图

```
strerror
  └─> __strerror_l (内部实现)
        ├─> ERROR_MESSAGES (静态错误消息表，编译时构建)
        └─> LCTRANS / __lctrans (locale 消息翻译)
              └─> locale 模块内部实现

strerror_l
  └─> __strerror_l (同 strerror，共享实现体)
```

---

## 内部数据结构

### 错误消息表

```rust
// Internal -- 编译时根据 __strerror.h 中 E(n, s) 宏展开构建
// 按错误码索引的静态 C 字符串数组
static ERROR_MESSAGES: &[&'static CStr] = &[
    c"No error information",             // [0]
    c"Illegal byte sequence",            // [EILSEQ]
    c"Domain error",                     // [EDOM]
    // ... 其余错误消息
    c"Key was rejected by service",      // [EKEYREJECTED]
];
```

[Visibility]: Internal -- 模块内部静态只读数据，不对外导出

- 使用 `&CStr` 或 `*const c_char` 存储，保持与 C 侧 ABI 一致性。
- 编译时从 `__strerror.h` 的 `E(n, s)` 列表构建。Rust 侧可通过构建脚本（`build.rs`）或声明宏（`macro_rules!`）在编译期生成。
- 对于未显式定义的错误码槽位，通过数组长度边界检查回退到索引 0 的消息。

### MIPS EDQUOT 兼容

```rust
// Internal -- 条件编译常量，仅在 MIPS 架构上定义
#[cfg(any(target_arch = "mips", target_arch = "mips64"))]
const EDQUOT: c_int = 109;
#[cfg(any(target_arch = "mips", target_arch = "mips64"))]
const EDQUOT_ORIG: c_int = 1133;
```

[Visibility]: Internal -- 模块私有常量

- **Intention**: MIPS 架构历史上将 `EDQUOT` 的值错误地定义为 1133（超出常见错误码范围），musl 内部将其重映射为 109。Rust 侧通过 `#[cfg]` 条件编译仅在 MIPS 架构上启用此逻辑。

---

## [RELY]

- **`LCTRANS` / `__lctrans`**: locale 消息翻译函数。签名为 `fn __lctrans(msg: *const c_char, loc: locale_t) -> *mut c_char;`。负责根据 `loc` 参数中的 locale 信息将原始英文错误消息翻译为对应语言的字符串。由 locale 模块提供。
- **`locale_t` 类型**: `*mut c_void` opaque 指针，定义于 locale 模块。
- **`CURRENT_LOCALE`**: 访问当前线程 locale 句柄的内部宏/函数，由 pthread / locale 模块提供。
- **`__strerror.h`**: 错误码到消息字符串的映射表，以 `E(n, s)` 宏形式列出所有标准 errno 错误码及其描述字符串。Rust 侧通过构建脚本或声明宏处理此文件内容。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn strerror(e: c_int) -> *mut c_char;`
  `extern "C" fn strerror_l(e: c_int, loc: locale_t) -> *mut c_char;`

本模块保证对外提供上述两个 ABI 兼容的函数符号。`strerror_l` 与 `__strerror_l` 为弱别名关系，行为完全一致。对于任意有效的 `c_int` 错误码和 `locale_t` locale 句柄，返回值始终为指向有效 NUL 结尾静态字符串的指针，符合 ISO C 和 POSIX.1-2008 strerror/strerror_l 语义。