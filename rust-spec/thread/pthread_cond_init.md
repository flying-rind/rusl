# pthread_cond_init -- Rust 接口归约

## 原始 C 接口
```c
int pthread_cond_init(pthread_cond_t *restrict c, const pthread_condattr_t *restrict a);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_cond_init(
    c: *mut pthread_cond_t,
    a: *const pthread_condattr_t,
) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
初始化条件变量对象。使用默认设置或指定的属性对象（时钟类型、进程共享）配置条件变量内部字段。

## 前置条件
- `c` 为非空指针（`!c.is_null()`），指向未初始化的 `pthread_cond_t` 对象
- `a` 可以为 `NULL`（使用默认属性），或指向有效的 `pthread_condattr_t` 对象

## 后置条件
- 始终返回 `0`（成功）
- `*c` 被零初始化（所有字段归零）
- 若 `a != NULL`：
  - `c._c_clock = a.__attr & 0x7FFF_FFFF`（从属性提取时钟 ID）
  - 若 `a.__attr >> 31`（进程共享标志），则 `c._c_shared = (void *)-1`（标记为进程共享）
- 若 `a == NULL`：
  - 时钟默认为 `0`（`CLOCK_REALTIME`）
  - 默认非进程共享（`_c_shared = NULL`）

## 不变量
- 初始化后的条件变量 `_c_clock` 字段为有效时钟 ID
- `_c_shared` 为 `null`（非共享）或 `usize::MAX as *mut c_void`（共享），此值用作布尔判断

## 算法
```rust
pub extern "C" fn pthread_cond_init(
    c: *mut pthread_cond_t,
    a: *const pthread_condattr_t,
) -> c_int {
    unsafe {
        // 零初始化全部字段
        core::ptr::write_bytes(c, 0u8, 1);
    }
    if !a.is_null() {
        unsafe {
            let attr = (*a).__attr;
            // PthreadCond 字段访问器由 pthread_impl 提供
            (*c).set_c_clock((attr & 0x7FFF_FFFF) as c_int);
            if (attr >> 31) != 0 {
                (*c).set_c_shared(usize::MAX as *mut c_void);  // 进程共享标记
            }
        }
    }
    0
}
```

## Rust 内部设计要点
- `pthread_cond_t` 为 `#[repr(C)]` 结构体，内部字段布局与 C 一致
- `PthreadCond` 提供 `c_seq()`, `c_clock()`, `c_lock()`, `c_head()`, `c_tail()` 等类型安全的字段访问方法（定义于 `pthread_impl` 模块）
- `set_c_shared` 方法封装写入 `_c_shared` 字段
- 使用 `core::ptr::write_bytes` 进行零初始化，语义明确
- `restrict` 语义在 Rust 中由所有权和借用规则自然保证（调用者传入独享指针）

---

/* Rely */
[RELY]
Predefined Types:
  pthread_cond_t                   // #[repr(C)] 条件变量类型，定义于 crate 内部
  pthread_condattr_t               // #[repr(C)] 条件变量属性类型
  core::ffi::c_int / c_uint       // Rust 核心库 C FFI 类型

Internal Module:
  pthread_impl::PthreadCond        // 提供类型安全的字段访问方法（c_clock, c_shared 等）

Predefined Functions:
  core::ptr::write_bytes           // 零初始化内存块

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_cond_init(c: *mut pthread_cond_t, a: *const pthread_condattr_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
