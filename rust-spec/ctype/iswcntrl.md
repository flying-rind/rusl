# iswcntrl 函数规约 (Rust)

## 复杂度分级: Level 2

---

## 符号可见性分析

| 符号 | C 签名 | 可见性 | 说明 |
|------|--------|--------|------|
| `iswcntrl` | `int iswcntrl(wint_t wc)` | **External** | POSIX/C 标准函数，必须保持 C ABI 兼容 |
| `iswcntrl_l` | `int iswcntrl_l(wint_t c, locale_t l)` | **External** | POSIX.1-2008 标准函数，必须保持 C ABI 兼容 |
| `__iswcntrl_l` | `int __iswcntrl_l(wint_t c, locale_t l)` | **Internal** | musl 内部实现函数（`__` 前缀），rusl 不对外导出。实现可直接内联到 `iswcntrl_l` 中 |

---

## 依赖图

```
iswcntrl (POSIX)          [extern "C" ABI]
iswcntrl_l (POSIX)        [extern "C" ABI]
  └── 无外部依赖，纯数学区间判断
```

---

## [RELY]

本函数的 Rust 实现不依赖任何外部模块或预定义函数。核心逻辑为四个无符号区间比较，可在单个函数体内完成。

若使用 `locale_t` 类型（用于 `iswcntrl_l`），需依赖：
  pub type locale_t = *mut core::ffi::c_void;
                                    // 依赖 1: locale_t 类型定义，来自 crate 公共类型模块
  pub type wint_t = core::ffi::c_uint;
                                    // 依赖 2: wint_t 类型定义，来自 crate 公共类型模块

---

## [GUARANTEE]

Exported Interface:
  pub unsafe extern "C" fn iswcntrl(wc: wint_t) -> c_int;
                                    // 判断宽字符是否为控制字符
                                    // [Visibility]: External -- C/POSIX 标准函数

  pub unsafe extern "C" fn iswcntrl_l(wc: wint_t, l: locale_t) -> c_int;
                                    // locale 感知的控制字符判断
                                    // [Visibility]: External -- POSIX.1-2008 标准函数

Internal Interface (不对外导出):
  pub(crate) fn __iswcntrl_l(wc: wint_t, l: locale_t) -> c_int;
                                    // 内部实现，供 iswctype_l 等内部模块调用
                                    // [Visibility]: Internal -- rusl crate 私有

---

## Level 1: 公共 API 规约

### iswcntrl (对外导出)

```rust
pub unsafe extern "C" fn iswcntrl(wc: wint_t) -> c_int;
```

**[Visibility]: External -- C/POSIX 标准函数。`<wctype.h>` 声明。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）

- **后置条件 (Postcondition)**:
  - **Case 1**: `wc` 是控制字符，满足以下任一条件:
    - `wc < 32`（C0 控制字符）
    - `wc` 在 `[0x7F, 0x9F]` 范围内（DEL + C1 控制字符）
    - `wc` 在 `[0x2028, 0x2029]` 范围内（行/段分隔符）
    - `wc` 在 `[0xFFF9, 0xFFFB]` 范围内（行间注释锚点）
    - 返回非零值。
  - **Case 2**: 其他字符
    - 返回 0。

- **不变量 (Invariant)**: 纯函数。无内部状态。线程安全。

- **Intent**: 判断宽字符是否为 Unicode 控制字符。覆盖 C0、C1、Unicode 行分隔符和特殊控制字符。

- **系统算法**:
  ```
  使用四个无符号区间检查:
    (wc as u32) < 32
    || (wc as u32).wrapping_sub(0x7f) < 33     // 0x7F-0x9F
    || (wc as u32).wrapping_sub(0x2028) < 2    // 0x2028-0x2029
    || (wc as u32).wrapping_sub(0xfff9) < 3    // 0xFFF9-0xFFFB
  O(1) 时间复杂度，无分支预测惩罚。Rust 中利用 `wrapping_sub` 保持与 C 无符号回绕语义一致。
  ```

---

### iswcntrl_l (对外导出)

```rust
pub unsafe extern "C" fn iswcntrl_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: External -- POSIX.1-2008 标准函数。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）
  - `l`: 类型为 `locale_t`，有效的 locale 句柄（或 `NULL` 表示 C locale）

- **后置条件 (Postcondition)**: 同 `iswcntrl`。在 C locale 下行为完全等价；在其他 locale 下由 locale 的 LC_CTYPE 类别决定字符分类。

- **不变量 (Invariant)**: 纯函数。无内部状态。线程安全。

- **Intent**: locale 感知的控制字符判断。在当前 rusl 实现中，非 C locale 支持可能为简化实现（直接退化为 C locale 行为），但接口必须保持完整 ABI 兼容。

---

## Level 3: 内部实现设计

### __iswcntrl_l (内部函数)

```rust
pub(crate) fn __iswcntrl_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: Internal -- rusl crate 私有。供 iswctype_l 及其他内部分类函数调用。**

- **前置条件**: 同 `iswcntrl_l`
- **后置条件**: 同 `iswcntrl_l`
- **Intent**: 内部实现函数。`iswcntrl` 等价于 `__iswcntrl_l(wc, core::ptr::null_mut())`。在 C locale 下四个区间判断即可完成分类。

---

## Rust 设计与 C 实现的关键差异

1. **类型映射**:
   - C `wint_t` -> Rust `wint_t = core::ffi::c_uint` (32-bit unsigned)
   - C `locale_t` -> Rust `locale_t = *mut core::ffi::c_void` (指针大小)
   - C 返回 `int` -> Rust `core::ffi::c_int`

2. **无符号运算**: C 的 `(unsigned)wc-0x7f < 33` 依赖于无符号整数回绕（wraparound）语义。Rust 中 `c_uint` 在 release 模式下默认也是 wrapping，但为明确表达意图，使用 `wrapping_sub()` 方法。

3. **无条件依赖**: 本函数不依赖任何外部模块、位图表或系统调用。可完全在 `#![no_std]` 环境中实现，仅需 `core` crate。

4. **`__` 前缀内部函数**: C 的 `__iswcntrl_l` 在 musl 中也是全局可见符号（通过 `weak_alias`），但在 rusl 中该符号仅为 `pub(crate)` 可见，不对外导出。外部调用者使用 `iswcntrl_l`。