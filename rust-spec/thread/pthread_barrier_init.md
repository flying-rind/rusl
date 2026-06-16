# pthread_barrier_init -- Rust 接口归约

## 原始 C 接口
```c
int pthread_barrier_init(pthread_barrier_t *restrict b, const pthread_barrierattr_t *restrict a, unsigned count);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_barrier_init(
    b: *mut pthread_barrier_t,
    a: *const pthread_barrierattr_t,
    count: core::ffi::c_uint,
) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
初始化屏障对象，设置参与屏障的线程数 `count`。若提供属性对象，根据属性设置进程共享标志。屏障要求 `count` 个线程全部到达后才能继续。

## 前置条件
- `b` 为非空指针（`!b.is_null()`），指向未初始化的 `pthread_barrier_t` 对象
- `a` 可以为 `NULL`（默认属性）或指向有效的 `pthread_barrierattr_t`
- `count > 0`

## 后置条件
- Case 1 `count - 1 <= i32::MAX - 1`（即 `count <= i32::MAX`）：
  - `b._b_limit = (count - 1) | attr.__attr`（其中 `attr.__attr` 在 `a == NULL` 时为 0）
  - 返回 `0`
- Case 2 `count - 1 > i32::MAX - 1`（即 `count > i32::MAX`）：
  - 返回 `EINVAL`
  - `*b` 不变

## 字段编码说明
- `_b_limit` 存储 `(count - 1) | attr.__attr`
- 进程共享屏障：`attr.__attr == 0x8000_0000`，因此 `_b_limit < 0`（最高位为 1），`pshared_barrier_wait` 通过 `_b_limit < 0` 检测
- 非进程共享屏障：`attr.__attr == 0`，因此 `_b_limit == count - 1 >= 0`

## 不变量
- 初始化后的 `_b_limit` 字段编码了 `count - 1` 和进程共享标志
- 其余字段（`_b_lock`, `_b_waiters`, `_b_count`, `_b_waiters2`）由零初始化

## 算法
```rust
pub extern "C" fn pthread_barrier_init(
    b: *mut pthread_barrier_t,
    a: *const pthread_barrierattr_t,
    count: c_uint,
) -> c_int {
    // count 溢出检查
    if count == 0 || (count as u32).wrapping_sub(1) > (i32::MAX as u32).wrapping_sub(1) {
        return EINVAL;
    }
    let attr_val = if a.is_null() { 0 } else { unsafe { (*a).__attr } };
    unsafe {
        // 零初始化全部字段
        core::ptr::write_bytes(b, 0u8, 1);
        (*b).set_b_limit(((count - 1) as c_int) | (attr_val as c_int));
    }
    0
}
```

## Rust 内部设计要点
- `PthreadBarrier` 提供 `b_limit()`, `b_lock()`, `b_count()`, `b_waiters()`, `b_waiters2()` 等类型安全的字段访问方法（定义于 `pthread_impl` 模块）
- `set_b_limit` 方法封装写入 `_b_limit` 字段
- 使用 `core::ptr::write_bytes` 进行零初始化
- 位运算 `|` 将 count-1 和 attr.__attr 合并编码到 `_b_limit` 中

---

/* Rely */
[RELY]
Predefined Types:
  pthread_barrier_t                // #[repr(C)] 屏障类型，定义于 crate 内部
  pthread_barrierattr_t            // #[repr(C)] 屏障属性类型
  core::ffi::c_int / c_uint       // Rust 核心库 C FFI 类型

Internal Module:
  pthread_impl::PthreadBarrier     // 提供类型安全的字段访问方法（b_limit, b_lock, b_count, b_waiters, b_waiters2 等）

Predefined Constants:
  EINVAL                           // POSIX 错误码（来自 <errno.h>）

Predefined Functions:
  core::ptr::write_bytes           // 零初始化内存块

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_barrier_init(b: *mut pthread_barrier_t, a: *const pthread_barrierattr_t, count: c_uint) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
