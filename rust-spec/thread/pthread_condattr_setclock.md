# pthread_condattr_setclock -- Rust 接口归约

## 原始 C 接口
```c
int pthread_condattr_setclock(pthread_condattr_t *a, clockid_t clk);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_condattr_setclock(
    a: *mut pthread_condattr_t,
    clk: clockid_t,
) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
设置条件变量超时等待使用的时钟类型（`CLOCK_REALTIME` 或 `CLOCK_MONOTONIC`）到属性对象中。

## 前置条件
- `a` 为非空指针（`!a.is_null()`），指向有效的 `pthread_condattr_t` 对象
- `clk` 为有效的时钟 ID

## 后置条件
- Case 1 时钟验证成功（`clk == 0` 或 `clk == 1`，即 `CLOCK_REALTIME` 或 `CLOCK_MONOTONIC`）：
  - `a.__attr` 的最高位（进程共享标志）保持不变
  - `a.__attr` 的低 31 位设置为 `clk`
  - 返回 `0`
- Case 2 时钟无效（`clk < 0` 或 `(clk as u32).wrapping_sub(2) < 2`）：
  - 返回 `EINVAL`
  - `a.__attr` 不变

## 不变量
- `a.__attr` 最高位的进程共享标志在设置时钟时不可丢失

## 算法
```rust
pub extern "C" fn pthread_condattr_setclock(a: *mut pthread_condattr_t, clk: clockid_t) -> c_int {
    // clk-2U < 2: 惯用技巧，允许 0 和 1，其余被拒绝
    if clk < 0 || ((clk as u32).wrapping_sub(2) < 2) {
        return EINVAL;
    }
    unsafe {
        let attr = &mut (*a).__attr;
        *attr = (*attr & 0x8000_0000) | (clk as c_uint);  // 保留最高位，设置低 31 位
    }
    0
}
```

## Rust 内部设计要点
- `clockid_t` 定义为 `c_int` 类型别名
- 使用 `wrapping_sub` 替代 C 的无符号减法惯用技巧，语义明确
- `EINVAL` 为 POSIX 错误码常量，定义于 crate 内部
- 仅在解引用裸指针时使用 `unsafe`

---

/* Rely */
[RELY]
Predefined Types:
  pthread_condattr_t               // #[repr(C)] 条件变量属性类型
  clockid_t                        // = c_int 时钟 ID 类型，来自 <time.h>
  core::ffi::c_int / c_uint       // Rust 核心库 C FFI 类型

Predefined Constants:
  EINVAL                           // POSIX 错误码（来自 <errno.h>）

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_condattr_setclock(a: *mut pthread_condattr_t, clk: clockid_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
