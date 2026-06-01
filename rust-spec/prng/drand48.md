# drand48 族 Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public — 对外导出 (extern "C" ABI)
#[no_mangle]
pub unsafe extern "C" fn drand48() -> f64;

#[no_mangle]
pub unsafe extern "C" fn erand48(xsubi: *mut u16) -> f64;

#[no_mangle]
pub unsafe extern "C" fn lrand48() -> i64;

#[no_mangle]
pub unsafe extern "C" fn nrand48(xsubi: *mut u16) -> i64;

#[no_mangle]
pub unsafe extern "C" fn mrand48() -> i64;

#[no_mangle]
pub unsafe extern "C" fn jrand48(xsubi: *mut u16) -> i64;

#[no_mangle]
pub unsafe extern "C" fn srand48(seedval: i64);

#[no_mangle]
pub unsafe extern "C" fn seed48(seed16v: *mut u16) -> *mut u16;

#[no_mangle]
pub unsafe extern "C" fn lcong48(p: *const u16);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- 使用全局状态的函数（drand48, lrand48, mrand48, srand48, seed48, lcong48）：无参数前置条件，依赖内部全局状态。
- 使用调用者种子的函数（erand48, nrand48, jrand48）：`xsubi` 指向 3 个 `u16` 的可读写缓冲区。

**[Post-condition]:**
- `drand48()`: 推进全局 LCG 一步，返回 [0.0, 1.0) 的 f64。
- `lrand48()`: 返回 [0, 2^31) 的非负值。
- `mrand48()`: 返回 [-2^31, 2^31) 的有符号值。
- `srand48(seedval)`: 初始化全局种子为 {0x330E, seedval & 0xFFFF, (seedval >> 16) & 0xFFFF}。
- `seed48(seed16v)`: 设置新种子并返回旧种子指针。
- `lcong48(p)`: 设置全部 7 个 LCG 参数。

### 不变量

**[Invariant]:** 全局状态非线程安全。erand48/nrand48/jrand48 使用调用者提供的状态，天然线程安全。

### 意图

实现 SUSv2/POSIX 48 位 LCG 族。默认参数: a=0x5DEECE66D, c=0xB。
核心迭代委托给内部函数 `__rand48_step`。

### 系统算法

```
drand48(): s = __rand48_step(...) ; return s as f64 / 2.0f64.powi(48)
lrand48(): s = __rand48_step(...) ; return ((s >> 17) & 0x7FFFFFFF) as i64
mrand48(): s = __rand48_step(...) ; return ((s >> 16) as i32) as i64
```