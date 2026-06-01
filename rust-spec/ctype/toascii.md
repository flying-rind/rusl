# toascii Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn toascii(c: core::ffi::c_int) -> core::ffi::c_int;
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int` (`core::ffi::c_int`)，任意整数值。

**[Post-condition]:**
返回 `c & 0x7f`（清除第 7 位及以上所有位），将值映射到 0-127 的 ASCII 范围。

### 不变量

**[Invariant]:**
- 纯函数。无内部状态。
- 输入输出均为 `c_int`，内部实现仅做一次按位与操作。

### 意图

将字符强制转换为 7 位 ASCII。**此函数已过时，不应在新代码中使用。** 保留仅为 BSD/POSIX 兼容性。

### 系统算法

```
return c & 0x7f;
时间复杂度 O(1)。
```

### Rust 内部实现要点

- 实现极简，直接 `return c & 0x7f`
- 无需任何内部状态或依赖
- 可标记为 `#[inline]` 以消除函数调用开销

---

/* Rely */
[RELY]
(无依赖)

[GUARANTEE]
Exported Interface:
  extern "C" fn toascii(c: core::ffi::c_int) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 toascii 符号