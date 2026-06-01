# iswctype / wctype 函数规约 (Rust)

## 复杂度分级: Level 2

---

## 符号可见性分析

| 符号 | C 签名 | 可见性 | 说明 |
|------|--------|--------|------|
| `iswctype` | `int iswctype(wint_t wc, wctype_t type)` | **External** | POSIX/C 标准函数，必须保持 C ABI 兼容 |
| `wctype` | `wctype_t wctype(const char *s)` | **External** | POSIX/C 标准函数，必须保持 C ABI 兼容 |
| `iswctype_l` | `int iswctype_l(wint_t c, wctype_t t, locale_t l)` | **External** | POSIX.1-2008 标准函数 |
| `wctype_l` | `wctype_t wctype_l(const char *s, locale_t l)` | **External** | POSIX.1-2008 标准函数 |
| `__iswctype_l` | `int __iswctype_l(wint_t c, wctype_t t, locale_t l)` | **Internal** | musl 内部实现（`__` 前缀），rusl 不对外导出 |
| `__wctype_l` | `wctype_t __wctype_l(const char *s, locale_t l)` | **Internal** | musl 内部实现（`__` 前缀），rusl 不对外导出 |
| `WCTYPE_ALNUM` 等常量 | `#define` 宏 (1-12) | **External** | POSIX 要求的分类标识符常量 |

---

## 依赖图

```
iswctype (POSIX)               [extern "C" ABI]
  └── __iswctype_l (内部)      [pub(crate)]
        ├── iswalpha / iswalpha_l          [依赖 1]
        ├── iswblank / iswblank_l          [依赖 2]
        ├── iswcntrl / iswcntrl_l          [依赖 3]
        ├── iswdigit / iswdigit_l          [依赖 4]
        ├── iswgraph / iswgraph_l          [依赖 5]
        ├── iswlower / iswlower_l          [依赖 6]
        ├── iswprint / iswprint_l          [依赖 7]
        ├── iswpunct / iswpunct_l          [依赖 8]
        ├── iswspace / iswspace_l          [依赖 9]
        ├── iswupper / iswupper_l          [依赖 10]
        └── iswxdigit / iswxdigit_l        [依赖 11]

wctype (POSIX)                 [extern "C" ABI]
  └── __wctype_l (内部)        [pub(crate)]
        └── 字符串比较逻辑（内部实现，O(n) 遍历 12 个分类名）
```

---

## [RELY]

Predefined Functions:
  pub unsafe extern "C" fn iswalpha(wc: wint_t) -> c_int;
                                    // 依赖 1: 宽字符字母分类
  pub unsafe extern "C" fn iswblank(wc: wint_t) -> c_int;
                                    // 依赖 2: 宽字符空白分类
  pub unsafe extern "C" fn iswcntrl(wc: wint_t) -> c_int;
                                    // 依赖 3: 宽字符控制字符分类
  pub unsafe extern "C" fn iswdigit(wc: wint_t) -> c_int;
                                    // 依赖 4: 宽字符数字分类
  pub unsafe extern "C" fn iswgraph(wc: wint_t) -> c_int;
                                    // 依赖 5: 宽字符图形字符分类
  pub unsafe extern "C" fn iswlower(wc: wint_t) -> c_int;
                                    // 依赖 6: 宽字符小写字母分类
  pub unsafe extern "C" fn iswprint(wc: wint_t) -> c_int;
                                    // 依赖 7: 宽字符可打印字符分类
  pub unsafe extern "C" fn iswpunct(wc: wint_t) -> c_int;
                                    // 依赖 8: 宽字符标点符号分类
  pub unsafe extern "C" fn iswspace(wc: wint_t) -> c_int;
                                    // 依赖 9: 宽字符空白字符分类
  pub unsafe extern "C" fn iswupper(wc: wint_t) -> c_int;
                                    // 依赖 10: 宽字符大写字母分类
  pub unsafe extern "C" fn iswxdigit(wc: wint_t) -> c_int;
                                    // 依赖 11: 宽字符十六进制数字分类

Predefined Types:
  pub type wint_t = core::ffi::c_uint;
                                    // 依赖 12: 宽字符类型
  pub type wctype_t = core::ffi::c_ulong;
                                    // 依赖 13: 字符分类标识符类型
  pub type locale_t = *mut core::ffi::c_void;
                                    // 依赖 14: locale 句柄类型

---

## [GUARANTEE]

Exported Interface:
  pub unsafe extern "C" fn iswctype(wc: wint_t, desc: wctype_t) -> c_int;
                                    // 通用宽字符分类函数
                                    // [Visibility]: External -- C/POSIX 标准函数

  pub unsafe extern "C" fn wctype(name: *const c_char) -> wctype_t;
                                    // 将分类名称字符串解析为分类标识符
                                    // [Visibility]: External -- C/POSIX 标准函数

  pub unsafe extern "C" fn iswctype_l(wc: wint_t, desc: wctype_t, l: locale_t) -> c_int;
                                    // locale 感知的通用宽字符分类
                                    // [Visibility]: External -- POSIX.1-2008 标准函数

  pub unsafe extern "C" fn wctype_l(name: *const c_char, l: locale_t) -> wctype_t;
                                    // locale 感知的分类名称解析
                                    // [Visibility]: External -- POSIX.1-2008 标准函数

Exported Constants:
  pub const WCTYPE_ALNUM: wctype_t = 1;
  pub const WCTYPE_ALPHA: wctype_t = 2;
  pub const WCTYPE_BLANK: wctype_t = 3;
  pub const WCTYPE_CNTRL: wctype_t = 4;
  pub const WCTYPE_DIGIT: wctype_t = 5;
  pub const WCTYPE_GRAPH: wctype_t = 6;
  pub const WCTYPE_LOWER: wctype_t = 7;
  pub const WCTYPE_PRINT: wctype_t = 8;
  pub const WCTYPE_PUNCT: wctype_t = 9;
  pub const WCTYPE_SPACE: wctype_t = 10;
  pub const WCTYPE_UPPER: wctype_t = 11;
  pub const WCTYPE_XDIGIT: wctype_t = 12;
                                    // 分类标识符常量，值与分类名数组中顺序严格对应
                                    // [Visibility]: External -- POSIX 标准要求

Internal Interface (不对外导出):
  pub(crate) fn __iswctype_l(wc: wint_t, desc: wctype_t, l: locale_t) -> c_int;
                                    // 内部实现，供其他内部分类函数复用
                                    // [Visibility]: Internal -- rusl crate 私有

  pub(crate) fn __wctype_l(name: *const c_char, l: locale_t) -> wctype_t;
                                    // 内部实现，供其他内部模块复用
                                    // [Visibility]: Internal -- rusl crate 私有

---

## Level 1: 公共 API 规约

### iswctype (对外导出)

```rust
pub unsafe extern "C" fn iswctype(wc: wint_t, desc: wctype_t) -> c_int;
```

**[Visibility]: External -- C/POSIX 标准函数。`<wctype.h>` 声明。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）
  - `desc`: 由 `wctype()` 返回的有效分类标识符。若 `desc` 无效（超出 1-12 范围），行为是实现定义的（musl 返回 0）

- **后置条件 (Postcondition)**:
  - **Case 1**: `desc` 匹配某个已知分类且 `wc` 属于该分类 -> 返回非零值
  - **Case 2**: `desc` 不匹配任何已知分类或 `wc` 不属于该分类 -> 返回 0

- **不变量 (Invariant)**: 纯函数。线程安全。`desc` 值与分类的对应关系不可变。

- **Intent**: 宽字符分类的通用化接口，将分类类型参数化。等价于 `__iswctype_l(wc, desc, NULL)`。

- **系统算法**:
  ```
  通过 match / 跳转表分发到对应的 isw* 函数:
    WCTYPE_ALNUM  -> iswalnum(wc)
    WCTYPE_ALPHA  -> iswalpha(wc)
    WCTYPE_BLANK  -> iswblank(wc)
    WCTYPE_CNTRL  -> iswcntrl(wc)
    WCTYPE_DIGIT  -> iswdigit(wc)
    WCTYPE_GRAPH  -> iswgraph(wc)
    WCTYPE_LOWER  -> iswlower(wc)
    WCTYPE_PRINT  -> iswprint(wc)
    WCTYPE_PUNCT  -> iswpunct(wc)
    WCTYPE_SPACE  -> iswspace(wc)
    WCTYPE_UPPER  -> iswupper(wc)
    WCTYPE_XDIGIT -> iswxdigit(wc)
    其他          -> 0
  O(1) 分发。Rust 中 match 语句编译器会生成跳转表。
  ```

---

### wctype (对外导出)

```rust
pub unsafe extern "C" fn wctype(name: *const c_char) -> wctype_t;
```

**[Visibility]: External -- C/POSIX 标准函数。`<wctype.h>` 声明。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `name`: 指向以 null 结尾的 C 字符串，内容为分类名称。若 `name` 为 `NULL`，行为未定义。

- **后置条件 (Postcondition)**:
  - **Case 1**: `name` 匹配已知分类名称 -> 返回该分类的标识符 (1-12)
  - **Case 2**: `name` 不匹配任何已知分类 -> 返回 0

已知分类名称（与常量一一对应）:
  "alnum", "alpha", "blank", "cntrl", "digit",
  "graph", "lower", "print", "punct", "space",
  "upper", "xdigit"

- **不变量 (Invariant)**: 纯函数。分类名称列表不可变。分类标识符与名称顺序严格对应（`WCTYPE_ALNUM=1` 对应 `names[0]`）。

- **Intent**: 将分类名称字符串解析为分类标识符，供 `iswctype` 使用。实现 `<wctype.h>` 的可扩展字符分类机制。

- **系统算法**:
  ```
  遍历固定的分类名称列表（12 个条目），逐字节比较:
    names: "alnum\0" "alpha\0" "blank\0" "cntrl\0" "digit\0"
           "graph\0" "lower\0" "print\0" "punct\0" "space\0"
           "upper\0" "xdigit\0"
    找到匹配 -> 返回索引+1
    未找到   -> 返回 0
  O(n) 时间复杂度，n=12。Rust 实现可使用 const 数组 & 循环比较。
  ```

---

### iswctype_l (对外导出)

```rust
pub unsafe extern "C" fn iswctype_l(wc: wint_t, desc: wctype_t, l: locale_t) -> c_int;
```

**[Visibility]: External -- POSIX.1-2008 标准函数。必须保持 C ABI 兼容。**

- **前置条件**: 同 `iswctype`，增加 locale 参数
- **后置条件**: 同 `iswctype`。在 C locale 下行为等价；在其他 locale 下由 locale 的 LC_CTYPE 类别决定。
- **Intent**: locale 感知的通用宽字符分类。

---

### wctype_l (对外导出)

```rust
pub unsafe extern "C" fn wctype_l(name: *const c_char, l: locale_t) -> wctype_t;
```

**[Visibility]: External -- POSIX.1-2008 标准函数。必须保持 C ABI 兼容。**

- **前置条件**: 同 `wctype`，增加 locale 参数
- **后置条件**: 同 `wctype`
- **Intent**: locale 感知的分类名称解析。

---

## Level 3: 内部实现设计

### __iswctype_l (内部函数)

```rust
pub(crate) fn __iswctype_l(wc: wint_t, desc: wctype_t, l: locale_t) -> c_int;
```

**[Visibility]: Internal -- rusl crate 私有。供需要直接以分类标识符进行判断的内部模块使用。**

- **前置条件**: 同 `iswctype_l`
- **后置条件**: 同 `iswctype_l`
- **Intent**: 内部实现。`iswctype` 等价于 `__iswctype_l(wc, desc, core::ptr::null_mut())`。内部通过 match 语句分发到 `__isw*_l` 内部函数（避免间接调用开销）。

### __wctype_l (内部函数)

```rust
pub(crate) fn __wctype_l(name: *const c_char, l: locale_t) -> wctype_t;
```

**[Visibility]: Internal -- rusl crate 私有。供内部模块直接使用。**

- **Intent**: `wctype` 等价于 `__wctype_l(name, core::ptr::null_mut())`。Rust 实现可在 `unsafe` 块内将 `name` 指针转换为 `&[u8]` 切片或逐字节读取，与静态分类名数组比较。

---

## Rust 设计与 C 实现的关键差异

1. **常量定义**: C 使用 `#define` 宏定义 WCTYPE_* 常量。Rust 使用 `pub const` 项，类型明确为 `wctype_t`。musl 中常量值 1-12 对应分类名在 `names` 字符串数组中的顺序（每 6 字节一个条目），Rust 实现可保持相同值映射以维持 ABI 兼容。

2. **分类分发**: C 使用 `switch` 语句分发到 `isw*` 函数。Rust 同样使用 `match` 语句，编译器自动生成跳转表，性能等价。注意 Rust `match` 在完整覆盖 1-12 后仍需通配分支处理无效 `desc` 值。

3. **字符串比较**: C 的 `wctype` 使用 `strcmp` 与固定 `names` 字符串比较。Rust `#![no_std]` 实现可使用 `for` 循环逐字节比较，或构造一个简单的内联比较函数。12 个条目每条目最长为 6 字节，线性搜索开销可忽略。

4. **`__` 前缀符号**: C 的 `__iswctype_l` / `__wctype_l` 在 musl 中是全局符号（`weak_alias` 到 `iswctype_l` / `wctype_l`）。rusl 中这些符号仅为 `pub(crate)` 可见，外部通过 POSIX 名称调用。

5. **`#![no_std]` 合规**: 仅依赖 `core`。所有字符串操作使用原始指针 + 字节比较，不依赖 `alloc` 或 `std`。