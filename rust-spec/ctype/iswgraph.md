# iswgraph 函数规约 (Rust)

## 复杂度分级: Level 1

---

## 符号可见性分析

| 符号 | C 签名 | 可见性 | 说明 |
|------|--------|--------|------|
| `iswgraph` | `int iswgraph(wint_t wc)` | **External** | POSIX/C 标准函数，必须保持 C ABI 兼容 |
| `iswgraph_l` | `int iswgraph_l(wint_t c, locale_t l)` | **External** | POSIX.1-2008 标准函数，必须保持 C ABI 兼容 |
| `__iswgraph_l` | `int __iswgraph_l(wint_t c, locale_t l)` | **Internal** | musl 内部实现（`__` 前缀），rusl 不对外导出 |

---

## 依赖图

```
iswgraph (POSIX)          [extern "C" ABI]
iswgraph_l (POSIX)        [extern "C" ABI]
  ├── iswprint / iswprint_l          [依赖 1: 可打印字符判断]
  └── iswspace / iswspace_l          [依赖 2: 空白字符判断]
```

---

## [RELY]

Predefined Functions:
  pub unsafe extern "C" fn iswprint(wc: wint_t) -> c_int;
                                    // 依赖 1: 判断宽字符是否为可打印字符
  pub unsafe extern "C" fn iswspace(wc: wint_t) -> c_int;
                                    // 依赖 2: 判断宽字符是否为空白字符

Predefined Types:
  pub type wint_t = core::ffi::c_uint;
                                    // 依赖 3: wint_t 类型定义
  pub type locale_t = *mut core::ffi::c_void;
                                    // 依赖 4: locale_t 类型定义

---

## [GUARANTEE]

Exported Interface:
  pub unsafe extern "C" fn iswgraph(wc: wint_t) -> c_int;
                                    // 判断宽字符是否为图形字符（可打印且非空白）
                                    // [Visibility]: External -- C/POSIX 标准函数

  pub unsafe extern "C" fn iswgraph_l(wc: wint_t, l: locale_t) -> c_int;
                                    // locale 感知的图形字符判断
                                    // [Visibility]: External -- POSIX.1-2008 标准函数

Internal Interface (不对外导出):
  pub(crate) fn __iswgraph_l(wc: wint_t, l: locale_t) -> c_int;
                                    // 内部实现，供 iswctype_l 调用
                                    // [Visibility]: Internal -- rusl crate 私有

---

## Level 1: 公共 API 规约

### iswgraph (对外导出)

```rust
pub unsafe extern "C" fn iswgraph(wc: wint_t) -> c_int;
```

**[Visibility]: External -- C/POSIX 标准函数。`<wctype.h>` 声明。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）

- **后置条件 (Postcondition)**:
  - **Case 1**: `wc` 是可打印且非空格的宽字符
    - 即 `iswprint(wc) != 0 && iswspace(wc) == 0`
    - 返回非零值。
  - **Case 2**: 其他字符或 `WEOF`
    - 返回 0。

- **不变量 (Invariant)**: 纯函数。组合 `iswprint` 和 `iswspace` 实现，自身不维护额外状态。

- **Intent**: 判断宽字符是否为图形字符（graph character）。按 ISO C 标准定义，等价于 `!iswspace(wc) && iswprint(wc)`，即排除空白字符的可打印字符。

- **系统算法**:
  ```
  短路求值:
    先调用 iswspace(wc)（通常为 O(1) 位运算或区间检查，比 iswprint 更便宜）
    若 iswspace 返回非零 -> 直接返回 0（短路）
    否则调用 iswprint(wc) -> 返回其结果
  
  先检查 iswspace 的短路策略: iswspace 仅需检查少数几个码点（空格、制表符等），
  而 iswprint 需要更复杂的区间判断。短路后可避免不必要的 iswprint 位图查找。
  ```

---

### iswgraph_l (对外导出)

```rust
pub unsafe extern "C" fn iswgraph_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: External -- POSIX.1-2008 标准函数。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）
  - `l`: 类型为 `locale_t`，有效的 locale 句柄（或 `NULL` 表示 C locale）

- **后置条件 (Postcondition)**: 同 `iswgraph`。等价于 `!iswspace_l(wc, l) && iswprint_l(wc, l)`。

- **不变量 (Invariant)**: 纯函数。线程安全。

- **Intent**: locale 感知的图形字符判断。

---

## Level 3: 内部实现设计

### __iswgraph_l (内部函数)

```rust
pub(crate) fn __iswgraph_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: Internal -- rusl crate 私有。**

- **前置条件**: 同 `iswgraph_l`
- **后置条件**: 同 `iswgraph_l`
- **Intent**: 内部实现。`iswgraph` 等价于 `__iswgraph_l(wc, core::ptr::null_mut())`。内部调用 `__iswspace_l` 和 `__iswprint_l`（内部版本），直接进行短路求值。

---

## Rust 设计与 C 实现的关键差异

1. **组合实现**: `iswgraph` 自身无独立算法，完全委托给 `iswprint` 和 `iswspace`。Rust 实现中可调用对应的 `pub(crate)` 内部版本 `__iswspace_l` 和 `__iswprint_l` 以减少一次间接调用。

2. **短路优化**: 保持与 C 相同的短路顺序（先 iswspace 后 iswprint）。Rust 的 `&&` 运算符天然支持短路求值。

3. **零额外依赖**: 仅依赖 `iswprint` 和 `iswspace` 两个外部函数，无位图或位表依赖。

4. **图形字符定义**: `graph = print - space`。这意味着:
   - 字母、数字、标点符号 -> 是图形字符
   - 空格、制表符、换行符等 -> 非图形字符
   - 控制字符 -> 非图形字符（同时也不属于 print）