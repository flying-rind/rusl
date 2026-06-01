# iswlower 函数规约 (Rust)

## 复杂度分级: Level 2

---

## 符号可见性分析

| 符号 | C 签名 | 可见性 | 说明 |
|------|--------|--------|------|
| `iswlower` | `int iswlower(wint_t wc)` | **External** | POSIX/C 标准函数，必须保持 C ABI 兼容 |
| `iswlower_l` | `int iswlower_l(wint_t c, locale_t l)` | **External** | POSIX.1-2008 标准函数，必须保持 C ABI 兼容 |
| `__iswlower_l` | `int __iswlower_l(wint_t c, locale_t l)` | **Internal** | musl 内部实现（`__` 前缀），rusl 不对外导出 |

---

## 依赖图

```
iswlower (POSIX)          [extern "C" ABI]
iswlower_l (POSIX)        [extern "C" ABI]
  └── towupper / towupper_l          [依赖 1: 大小写转换表]
        └── casemap 大小写映射表     [内部数据结构]
```

---

## [RELY]

Predefined Functions:
  pub unsafe extern "C" fn towupper(wc: wint_t) -> wint_t;
                                    // 依赖 1: 将宽字符转换为对应的大写形式
                                    // 核心语义: 若 wc 是小写字母，返回其大写映射；
                                    // 否则返回 wc 自身。iswlower 通过检测
                                    // towupper(wc) != wc 来判定是否为小写字母

Predefined Types:
  pub type wint_t = core::ffi::c_uint;
                                    // 依赖 2: wint_t 类型定义
  pub type locale_t = *mut core::ffi::c_void;
                                    // 依赖 3: locale_t 类型定义

---

## [GUARANTEE]

Exported Interface:
  pub unsafe extern "C" fn iswlower(wc: wint_t) -> c_int;
                                    // 判断宽字符是否为小写字母
                                    // [Visibility]: External -- C/POSIX 标准函数

  pub unsafe extern "C" fn iswlower_l(wc: wint_t, l: locale_t) -> c_int;
                                    // locale 感知的小写字母判断
                                    // [Visibility]: External -- POSIX.1-2008 标准函数

Internal Interface (不对外导出):
  pub(crate) fn __iswlower_l(wc: wint_t, l: locale_t) -> c_int;
                                    // 内部实现，供 iswctype_l 调用
                                    // [Visibility]: Internal -- rusl crate 私有

---

## Level 1: 公共 API 规约

### iswlower (对外导出)

```rust
pub unsafe extern "C" fn iswlower(wc: wint_t) -> c_int;
```

**[Visibility]: External -- C/POSIX 标准函数。`<wctype.h>` 声明。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）

- **后置条件 (Postcondition)**:
  - **Case 1**: `wc` 是小写字母
    - 即 `towupper(wc) != wc`（存在对应的大写映射且映射结果不等于自身）
    - 返回非零值。
  - **Case 2**: `wc` 不是小写字母（无对应大写或 `wc == WEOF`）
    - 返回 0。
    
    **关键边界情况**:
    - 大写字母 `A-Z`: `towupper('A') == 'A'`，返回 0（非小写）
    - 无大小写字符（如数字、标点）: `towupper('1') == '1'`，返回 0
    - `WEOF`: `towupper(WEOF) == WEOF`，返回 0
    - 小写字母 `a-z` 及其它 Unicode 小写字母: `towupper('a') == 'A' != 'a'`，返回非零

- **不变量 (Invariant)**: 纯函数。依赖 `towupper` 的大小写映射表，无自身独立状态。

- **Intent**: 通过检测 `towupper(wc) != wc` 判断宽字符是否为小写字母。这种设计的精妙之处在于避免了维护独立的小写字母分类表——直接利用大小写转换表反向推断。

- **系统算法**:
  ```
  单次比较:
    若 towupper(wc) != wc -> 返回非零（是小写字母）
    否则 -> 返回 0（非小写字母）
  
  时间复杂度取决于 towupper 的实现。对于 ASCII 字符，towupper 通常为 O(1)
  区间判断（如 wc 在 'a'..'z' 内则加偏移量）；对于非 ASCII 字符，取决于
  casemap 查找表的实现（通常 O(1) 的分段位图查找）。
  ```

---

### iswlower_l (对外导出)

```rust
pub unsafe extern "C" fn iswlower_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: External -- POSIX.1-2008 标准函数。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）
  - `l`: 类型为 `locale_t`，有效的 locale 句柄（或 `NULL` 表示 C locale）

- **后置条件 (Postcondition)**: 同 `iswlower`。等价于 `towupper_l(wc, l) != wc`。

- **不变量 (Invariant)**: 纯函数。线程安全。

- **Intent**: locale 感知的小写字母判断。某些 locale 可能额外定义小写字母（如带变音符号的字母）。

---

## Level 3: 内部实现设计

### __iswlower_l (内部函数)

```rust
pub(crate) fn __iswlower_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: Internal -- rusl crate 私有。**

- **前置条件**: 同 `iswlower_l`
- **后置条件**: 同 `iswlower_l`
- **Intent**: 内部实现。`iswlower` 等价于 `__iswlower_l(wc, core::ptr::null_mut())`。内部调用 `__towupper_l(wc, l)` 获取大写映射，然后与 `wc` 比较。

- **Rust 实现要点**:
  ```rust
  // 核心逻辑
  if __towupper_l(wc, l) != wc { 1 } else { 0 }
  ```
  注意 `towupper` 返回 `wint_t` 类型，比较时使用 `!=` 即可，结果自然转为 `c_int`。

---

## Rust 设计与 C 实现的关键差异

1. **逆转发策略**: `iswlower` 是典型的"通过转换表反向推断"设计模式。这种策略的优势：
   - 避免维护独立的小写分类表（节省空间）
   - 自动与 `towupper` 保持一致（若 `towupper` 的映射表更新，`iswlower` 自动适应）
   - 判定语义清晰：字符是小写字母当且仅当它有不同的大写形式

2. **与 isupper 的对称性**: 
   - `iswlower(wc)` = `towupper(wc) != wc`
   - `iswupper(wc)` = `towlower(wc) != wc`
   两者完全对称，共享同一设计哲学。

3. **towupper 依赖**: `towupper` 内部依赖 casemap 映射表（通常为分段位图或区间表）。Rust 实现中，casemap 可作为 `pub(crate) static` 查找表定义在独立模块中。

4. **`__` 前缀符号**: `__iswlower_l` 在 rusl 中为 `pub(crate)` 可见，不对外导出。外部通过 `iswlower_l` 调用。

5. **`WEOF` 处理**: `towupper(WEOF)` 返回 `WEOF`，因此 `towupper(WEOF) != WEOF` 为假，`iswlower(WEOF)` 正确返回 0。无需特殊分支。