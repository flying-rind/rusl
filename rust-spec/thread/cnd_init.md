# cnd_init — Rust 接口归约

## 原始 C 接口
```c
int cnd_init(cnd_t *c);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.1)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn cnd_init(c: *mut cnd_t) -> core::ffi::c_int;
```

---

## 意图
将条件变量 `c` 初始化为全零状态，即为进程内使用的私有条件变量。零初始化等价于 POSIX 默认属性。

## 前置条件
- `c` 为非空指针（`!c.is_null()`），指向有效的内存位置
- `*c` 之前未被初始化（或已被销毁且不再使用）

## 后置条件
- `*c` 的所有字节设为 0（通过零初始化）
- 始终返回 `thrd_success` (0)
- 初始化后的条件变量可用于 `cnd_wait`、`cnd_timedwait`、`cnd_signal`、`cnd_broadcast`

## 不变量
- 零初始化产生默认属性（私有、进程内条件变量）

## 算法
```rust
extern "C" fn cnd_init(c: *mut cnd_t) -> c_int {
    // 零初始化：底层 cnd_t 结构的所有字段置零
    unsafe {
        core::ptr::write_bytes(c as *mut u8, 0, core::mem::size_of::<cnd_t>());
    }
    thrd_success
}
```

注意：`cnd_t` 等同于 `pthread_cond_t`，musl 中为包含 12 个 `int` 的 union struct。零初始化等价于 `PTHREAD_COND_INITIALIZER`。

---

## Rust 内部辅助接口（模块私有）

```rust
// cnd_t 的 Rust 类型表示（内部使用，与 C 布局兼容的 repr(C) struct）
#[repr(C)]
pub(crate) struct CndT {
    // 内部字段与 pthread_cond_t 布局完全一致
    __u: CndUnion,
}

impl CndT {
    pub(crate) fn new() -> Self {
        // 零初始化，等价于 PTHREAD_COND_INITIALIZER
        unsafe { core::mem::zeroed() }
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::write_bytes              // 依赖1: Rust 核心库批量内存写入
  core::mem::size_of / core::mem::zeroed  // 依赖2: 大小计算与零初始化
Predefined Macros/Types:
  cnd_t (= pthread_cond_t)            // C11 条件变量类型（repr(C)，与 POSIX 布局兼容）
  thrd_success (= 0)                  // C11 成功返回值

[GUARANTEE]
Exported Interface:
  extern "C" fn cnd_init(c: *mut cnd_t) -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 cnd_init 符号
Internal Interface:
  (无内部导出)
