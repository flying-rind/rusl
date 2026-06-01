# iswprint 函数规约 (Rust)

## 复杂度分级: Level 3

---

## 符号可见性分析

| 符号 | C 签名 | 可见性 | 说明 |
|------|--------|--------|------|
| `iswprint` | `int iswprint(wint_t wc)` | **External** | POSIX/C 标准函数，必须保持 C ABI 兼容 |
| `iswprint_l` | `int iswprint_l(wint_t c, locale_t l)` | **External** | POSIX.1-2008 标准函数，必须保持 C ABI 兼容 |
| `__iswprint_l` | `int __iswprint_l(wint_t c, locale_t l)` | **Internal** | musl 内部实现（`__` 前缀），rusl 不对外导出 |

---

## 依赖图

```
iswprint (POSIX)          [extern "C" ABI]
iswprint_l (POSIX)        [extern "C" ABI]
  └── 无外部依赖，纯区间判断逻辑
      内部使用分段区间比较（5 阶段决策树），O(1) 时间复杂度
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
  pub unsafe extern "C" fn iswprint(wc: wint_t) -> c_int;
                                    // 判断宽字符是否为可打印字符
                                    // [Visibility]: External -- C/POSIX 标准函数

  pub unsafe extern "C" fn iswprint_l(wc: wint_t, l: locale_t) -> c_int;
                                    // locale 感知的可打印字符判断
                                    // [Visibility]: External -- POSIX.1-2008 标准函数

Internal Interface (不对外导出):
  pub(crate) fn __iswprint_l(wc: wint_t, l: locale_t) -> c_int;
                                    // 内部实现，供 iswctype_l 及 iswgraph 调用
                                    // [Visibility]: Internal -- rusl crate 私有

---

## Level 1: 公共 API 规约

### iswprint (对外导出)

```rust
pub unsafe extern "C" fn iswprint(wc: wint_t) -> c_int;
```

**[Visibility]: External -- C/POSIX 标准函数。`<wctype.h>` 声明。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）

- **后置条件 (Postcondition)**:
  - **Case 1**: `wc` 是可打印字符 -> 返回非零值
  
    可打印字符的判定逻辑（按优先级顺序）:
    1. `wc < 0xff` 且 `(wc+1 & 0x7f) >= 0x21`
       - 即低 7 位在 [0x20, 0x7E] 范围内（ASCII 可打印，不含 DEL）
    2. `wc < 0x2028`
       - BMP 中排除 C0/C1 控制字符后的所有码点
    3. `wc` 在 `[0x202A, 0xD7FF]` 范围内
       - 排除行/段分隔符（U+2028-U+2029）
    4. `wc` 在 `[0xE000, 0xFFF8]` 范围内
       - 私用区及其它可打印字符

  - **Case 2**: `wc` 不是可打印字符 -> 返回 0
    
    非可打印字符包括:
    - `wc >= 0xfffc`（非字符码点及越界码点）
    - 或 `(wc & 0xfffe) == 0xfffe`（非字符码点 U+FFFE、U+FFFF 及高位面对应码点）
    - C0 控制字符 (U+0000-U+001F)
    - DEL (U+007F) 及 C1 控制字符 (U+0080-U+009F)
    - 行/段分隔符 (U+2028-U+2029)
    - 行间注释锚点 (U+FFF9-U+FFFB)
    - `WEOF` (通常为 0xFFFFFFFF)

- **不变量 (Invariant)**: 纯函数。无内部状态。线程安全。

- **Intent**: 判断宽字符是否为可打印字符。排除所有控制字符、非字符码点和特殊分隔符。实现针对常见可打印字符（ASCII 可打印 + BMP 普通字符）的热路径进行了高度优化，采用分段区间决策树。

- **系统算法**:
  ```
  五阶段决策树（优先级从高到低）:

  Phase 1: wc < 0xFF 时
    使用位运算检查低 7 位是否在 [0x20, 0x7E]:
      (wc + 1) & 0x7F >= 0x21
    解释: wc+1 使 0x1F 变为 0x20，0x7F 变为 0x80；
           & 0x7F 取低 7 位；>= 0x21 表示原值在 [0x20, 0x7E]。
    ASCII 可打印字符热路径，非常快。

  Phase 2: wc < 0x2028
    直接返回真。
    覆盖 BMP 中 0xFF-0x2027 范围（排除 C1 控制字符 0x80-0x9F
    属于第一层判断，剩余为非控制字符）。

  Phase 3: wc 在 [0x202A, 0xD7FF] 或 [0xE000, 0xFFF8]
    返回真。
    覆盖 BMP 中排除行分隔符和代理区、私用区后的可打印范围。

  Phase 4: wc >= 0xFFFC
    返回假。
    排除: U+FFFC (对象替换字符), U+FFFD (替换字符),
          U+FFFE (非字符), U+FFFF (非字符) 及所有 > U+FFFF 的码点。

  Phase 5: (wc & 0xFFFE) == 0xFFFE
    返回假。
    排除所有以 FFFE/FFFF 结尾的非字符码点（如 U+1FFFE, U+2FFFE 等）。
    此判断需在 Phase 4 之后执行（不重叠）。

  Phase 6: 其余
    返回真。
    覆盖 CJK 扩展、高位平面字符等（如 U+20000-U+2FFFD 等）。
    这些码点 > 0xFFFF 且通过 Phase 4/5 的排除。

  总时间复杂度 O(1)，最坏情况 6 个分支。
  ```

---

### iswprint_l (对外导出)

```rust
pub unsafe extern "C" fn iswprint_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: External -- POSIX.1-2008 标准函数。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）
  - `l`: 类型为 `locale_t`，有效的 locale 句柄（或 `NULL` 表示 C locale）

- **后置条件 (Postcondition)**: 同 `iswprint`。

- **Intent**: locale 感知的可打印字符判断。

---

## Level 3: 内部实现设计

### __iswprint_l (内部函数)

```rust
pub(crate) fn __iswprint_l(wc: wint_t, l: locale_t) -> c_int;
```

**[Visibility]: Internal -- rusl crate 私有。供 iswctype_l 及 iswgraph 内部实现调用。**

- **前置条件**: 同 `iswprint_l`
- **后置条件**: 同 `iswprint_l`
- **Intent**: 内部实现。`iswprint` 等价于 `__iswprint_l(wc, core::ptr::null_mut())`。

- **Rust 实现要点**:
  ```rust
  pub(crate) fn __iswprint_l(wc: wint_t, _l: locale_t) -> c_int {
      let w = wc as u32;
      if w < 0xff {
          return ((w + 1) & 0x7f >= 0x21) as c_int;
      }
      if w < 0x2028 {
          return 1;
      }
      if (w >= 0x202A && w <= 0xD7FF) || (w >= 0xE000 && w <= 0xFFF8) {
          return 1;
      }
      if w >= 0xFFFC {
          return 0;
      }
      if w & 0xFFFE == 0xFFFE {
          return 0;
      }
      1 // 其余高位平面字符
  }
  ```
  
  **注意**: Rust 的 `bool as c_int` 转换（`true` -> 1, `false` -> 0）是安全的，与 C 的布尔到 int 转换语义一致。

---

## Unicode 码点分类表（参考）

| 码点范围 | 分类 | iswprint 结果 | 说明 |
|----------|------|---------------|------|
| U+0000..U+001F | C0 控制字符 | 0 | NULL, SOH, ..., US |
| U+0020..U+007E | ASCII 可打印 | 非零 | 空格 + 所有可打印 ASCII |
| U+007F | DEL | 0 | 删除字符 |
| U+0080..U+009F | C1 控制字符 | 0 | ISO 6429 控制字符 |
| U+00A0..U+2027 | 可打印 | 非零 | 拉丁扩展、符号等 |
| U+2028..U+2029 | 行/段分隔符 | 0 | LINE SEPARATOR, PARAGRAPH SEPARATOR |
| U+202A..U+D7FF | 可打印 | 非零 | 双向控制 + 各类文字 |
| U+D800..U+DFFF | 代理区 | 0 | UTF-16 代理对（非有效 Unicode 标量值） |
| U+E000..U+FFF8 | 私用区等 | 非零 | PUA + 特殊符号 |
| U+FFF9..U+FFFB | 行间注释锚点 | 0 | INTERLINEAR ANNOTATION ANCHOR/SEPARATOR/TERMINATOR |
| U+FFFC..U+FFFD | 替换字符 | 0 | OBJECT REPLACEMENT / REPLACEMENT CHARACTER |
| U+FFFE..U+FFFF | 非字符 | 0 | 永久保留的非字符码点 |
| U+10000..U+10FFFD | 补充平面 | 非零 | 除 U+xFFFE/U+xFFFF 外的所有有效码点 |

---

## Rust 设计与 C 实现的关键差异

1. **区间判断**: C 使用 `(unsigned)(wc-0x7f) < 33` 风格的无符号减法区间检查。Rust 可直接使用 `w >= start && w <= end` 形式的比较，编译器会将其优化为等价的区间检查指令。对于更紧凑的位运算（如 Phase 1 的 `(wc+1) & 0x7F >= 0x21`），Rust 逐位操作语义与 C 一致。

2. **非字符码点排除**: `(wc & 0xFFFE) == 0xFFFE` 是精巧的位运算技巧，可同时检测 U+FFFE/U+FFFF 以及所有高位平面的对应非字符码点（U+1FFFE, U+2FFFE, ... U+10FFFE/U+10FFFF）。Rust 保持相同的位运算逻辑。

3. **代理区排除**: U+D800..U+DFFF 被 Phase 3 的区间 [0x202A, 0xD7FF] 和 [0xE000, 0xFFF8] 之间的间隙自然排除，无需单独判断。

4. **分支优化**: Rust 编译器（LLVM 后端）会将连续的 `if` 链优化为与 C 等价的跳转表或条件移动指令。Phase 1 的位运算热路径尤其适合现代 CPU 的分支预测。

5. **零依赖**: 不依赖任何外部函数、位图表或系统调用。仅需 `core` crate 即可完成全部逻辑。