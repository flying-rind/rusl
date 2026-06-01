# isalpha 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```rust
use core::ffi::{c_int, c_void};

// locale_t 为 opaque 指针类型，在 Rust 侧用 *mut c_void 表示以保持 ABI 兼容
type locale_t = *mut c_void;

extern "C" fn isalpha(c: c_int) -> c_int;

extern "C" fn __isalpha_l(c: c_int, l: locale_t) -> c_int;

// weak_alias: isalpha_l 是 __isalpha_l 的弱别名，共享同一实现
extern "C" fn isalpha_l(c: c_int, l: locale_t) -> c_int;
```

### 前置/后置条件

**[Pre-condition]:**
- `isalpha(c)`: `c` 类型为 `c_int`，值必须可表示为 `c_uchar` 或等于 `EOF`（通常为 -1）。
- `__isalpha_l(c, l)`: `c` 同上；`l` 为有效的 `locale_t` 句柄（可为 `NULL` 表示 C locale）。
- `isalpha_l(c, l)`: `c` 同上；`l` 必须是非 `NULL` 的有效 `locale_t` 句柄，或为 `LC_GLOBAL_LOCALE` 特殊值。

**[Post-condition]:**
- Case 1: `c` 可表示为 `c_uchar`，且（`c` 自身或 `c|32` 转小写后）在 `'a'`-`'z'` 范围内
  - 返回非零值（具体值由实现定义，通常为当前 locale 下该字符的位掩码或 1）。
- Case 2: `c` 不是字母，或 `c == EOF`
  - 返回 0。
- 对于 `__isalpha_l` 和 `isalpha_l`，`l` 参数在此实现中被忽略（始终使用 C locale 规则），行为与 `isalpha(c)` 一致。

### 不变量

**[Invariant]:** 纯函数。无内部可变状态。此 musl 实现不依赖 locale 参数，`__isalpha_l` 和 `isalpha_l` 内部均回退到与 `isalpha` 相同的 C locale 位运算逻辑。

### 意图

判断字符是否为英文字母（`'a'`-`'z'` 或 `'A'`-`'Z'`）。使用无分支位运算 `((unsigned)c|32)-'a' < 26`，将大写字母通过 `|32` 转为小写后统一比较，消除条件分支。

在 Rust 内部实现中，可选用同等位运算逻辑，或将 locale 参数视为预留扩展（当前实现忽略）。`isalpha_l` 通过 `weak_alias!` 宏与 `__isalpha_l` 共享同一函数体。

### 系统算法

```
将 c 转为 unsigned 类型，通过 |32 将大写字母转为小写，
然后判断是否在 'a' 到 'z' 范围内（差小于 26）。
时间复杂度 O(1)，无分支。
```