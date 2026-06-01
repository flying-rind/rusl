# iswdigit 函数规约 (Rust)

## 复杂度分级: Level 1

---

## 符号可见性分析

| 符号 | C 签名 | 可见性 | 说明 |
|------|--------|--------|------|
| `iswdigit` | `int iswdigit(wint_t wc)` | **External** | POSIX/C 标准函数，必须保持 C ABI 兼容 |
| `iswdigit_l` | `int iswdigit_l(wint_t c, locale_t l)` | **External** | POSIX.1-2008 标准函数，必须保持 C ABI 兼容 |
| `__iswdigit_l` | `int __iswdigit_l(wint_t c, locale_t l)` | **Internal** | musl 内部实现（`__` 前缀），rusl 不对外导出 |

---

## 依赖图

```
iswdigit (POSIX)          [extern "C" ABI]
iswdigit_l (POSIX)        [extern "C" ABI]
  └── 无外部依赖，单行无符号区间判断
```

---

## [RELY]

Predefined Types:
  pub type wint_t = core::ffi::c_uint;
                                    // 依赖 1: wint_t 类型定义，来自 crate 公共类型模块
  pub type locale_t = *mut core::ffi::c_void;
                                    // 依赖 2: locale_t 类型定义，来自 crate 公共类型模块

---

## [GUARANTEE]

Exported Interface:
  pub unsafe extern "C" fn iswdigit(wc: wint_t) -> c_int;
                                    // 判断宽字符是否为十进制数字字符
                                    // [Visibility]: External -- C/POSIX 标准函数

  pub unsafe extern "C" fn iswdigit_l(wc: wint_t, l: locale_t) -> c_int;
                                    // locale 感知的十进制数字字符判断
                                    // [Visibility]: External -- POSIX.1-2008 标准函数

Internal Interface (不对外导出):
  pub(crate) fn __iswdigit_l(wc: wint_t, l: locale_t) -> c_int;
                                    // 内部实现，供 iswctype_l 及其他内部分类函数调用
                                    // [Visibility]: Internal -- rusl crate 私有

---

## Level 1: 公共 API 规约

### iswdigit (对外导出)

```rust
pub unsafe extern "C" fn iswdigit(wc: wint_t) -> c_int;
```

**[Visibility]: External -- C/POSIX 标准函数。`<wctype.h>` 声明。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）

- **后置条件 (Postcondition)**:
  - **Case 1**: `wc` 是十进制数字字符（`L'0'` 至 `L'9'`，即 Unicode 码点 U+0030 至 U+0039）
    - 返回非零值。
  - **Case 2**: 其他字符或 `WEOF`
    - 返回 0。

- **不变量 (Invariant)**: 纯函数。无内部状态。线程安全。

- **Intent**: 判断宽字符是否为十进制数字字符。宽字符数字在 BMP 中与 ASCII 数字同码点值（U+0030-U+0039），利用无符号区间检查 O(1) 一次判定。

- **系统算法**:
  ```
  单行无符号区间检查:
    (wc as u32).wrapping_sub('0' as u32) < 10
  O(1) 时间复杂度，无分支。Rust 中利用 wrapping_sub 保持无符号回绕语义，
  当 wc < '0' 时 wrapping_sub 产生大值，自然不满足 < 10 条件。
  ```

---

### iswdigit_l (对外导出)

```rust
pub unsafe extern "C" fn iswdigit_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: External -- POSIX.1-2008 标准函数。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）
  - `l`: 类型为 `locale_t`，有效的 locale 句柄（或 `NULL` 表示 C locale）

- **后置条件 (Postcondition)**: 同 `iswdigit`。在 C locale 下行为完全等价。

- **不变量 (Invariant)**: 纯函数。线程安全。

- **Intent**: locale 感知的数字字符判断。某些 locale 可能包含非 ASCII 数字字符（如阿拉伯数字 U+0660-U+0669），但当前 rusl 简化实现仅覆盖 ASCII 数字。

---

## Level 3: 内部实现设计

### __iswdigit_l (内部函数)

```rust
pub(crate) fn __iswdigit_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: Internal -- rusl crate 私有。供 iswctype_l 等内部分类函数调用。**

- **前置条件**: 同 `iswdigit_l`
- **后置条件**: 同 `iswdigit_l`
- **Intent**: 内部实现。`iswdigit` 等价于 `__iswdigit_l(wc, core::ptr::null_mut())`。

---

## Rust 设计与 C 实现的关键差异

1. **无符号运算**: C 的 `(unsigned)wc-'0' < 10` 依赖无符号回绕。Rust 使用 `(wc as u32).wrapping_sub('0' as u32) < 10` 明确表达该意图。

2. **零依赖**: 不依赖任何外部模块、位图表或系统调用。仅需 `core` crate。

3. **`__` 前缀内部函数**: `__iswdigit_l` 在 rusl 中为 `pub(crate)` 可见，不对外导出。

4. **与 isdigit 的关系**: `iswdigit(L'x')` 与对应的 `isdigit('x')` 结果一致（ASCII 范围内），因 BMP 中数字码点与 ASCII 数字相同。