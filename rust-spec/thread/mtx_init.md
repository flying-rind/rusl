# mtx_init — Rust 接口归约

## 原始 C 接口
```c
int mtx_init(mtx_t *m, int type);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.1)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn mtx_init(m: *mut mtx_t, type_: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
根据 `type_` 参数将互斥锁初始化为普通互斥锁或递归互斥锁。零初始化所有字段，其中互斥锁类型字段 `_m_type` 按 C11 -> POSIX 映射设置。

## 前置条件
- `m` 为非空指针（`!m.is_null()`），指向有效的内存位置
- `*m` 之前未被初始化（或已被销毁且不再使用）
- `type_` 为 `mtx_plain` (0) 或 `mtx_recursive` (1)，通过 `type_ & mtx_recursive` 位测试判断

## 后置条件
- `*m` 被零初始化，且：
  - `_m_type = PTHREAD_MUTEX_NORMAL (0)` 当 `type_ & mtx_recursive == 0`（即 `mtx_plain`）
  - `_m_type = PTHREAD_MUTEX_RECURSIVE (1)` 当 `type_ & mtx_recursive != 0`（即 `mtx_recursive`）
  - 其他字段（如 `_m_lock`）均为 0
- 始终返回 `thrd_success` (0)
- 注意：`mtx_timed` 标志在初始化时被忽略，超时锁功能由 `mtx_timedlock` 直接支持

## 不变量
- 零初始化 `_m_lock == 0` 表示互斥锁未被持有
- `_m_type` 决定了 `mtx_lock` / `mtx_trylock` 的锁行为路径

## 算法
```rust
extern "C" fn mtx_init(m: *mut mtx_t, type_: c_int) -> c_int {
    unsafe {
        // 零初始化整个结构
        core::ptr::write_bytes(m as *mut u8, 0, core::mem::size_of::<mtx_t>());
        // 设置互斥锁类型字段
        let mtx_type = if (type_ & mtx_recursive) != 0 {
            PTHREAD_MUTEX_RECURSIVE
        } else {
            PTHREAD_MUTEX_NORMAL
        };
        // 通过 _m_type 字段偏移写入（与 C 布局一致）
        (*m).set_m_type(mtx_type);
    }
    thrd_success
}
```

---

## Rust 内部辅助接口（模块私有）

```rust
// mtx_t 的 Rust repr(C) 表示（内部使用）
#[repr(C)]
pub(crate) struct MtxT {
    // 内部字段与 pthread_mutex_t 布局完全一致
    __u: MtxUnion,  // union of int[_M_COUNT] arrays
}

impl MtxT {
    pub(crate) fn m_type(&self) -> c_int;
    pub(crate) fn set_m_type(&mut self, t: c_int);
    pub(crate) fn m_lock(&self) -> c_int;
    pub(crate) fn set_m_lock(&mut self, v: c_int);
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::write_bytes / core::mem::size_of  // 依赖1: Rust 核心库内存操作
Predefined Macros/Types:
  mtx_t (= pthread_mutex_t, repr(C) union struct)  // C11 互斥锁类型
  PTHREAD_MUTEX_NORMAL (0) / PTHREAD_MUTEX_RECURSIVE (1)
  mtx_plain (0) / mtx_recursive (1)       // C11 互斥锁类型枚举
  thrd_success (= 0)                       // C11 成功返回值
  _m_type / _m_lock                        // 互斥锁结构字段访问（偏移与 C 宏一致）

[GUARANTEE]
Exported Interface:
  extern "C" fn mtx_init(m: *mut mtx_t, type_: core::ffi::c_int) -> core::ffi::c_int;
                                           // 本模块保证对外提供与 C ABI 兼容的 mtx_init 符号
Internal Interface:
  (无内部导出 — MtxT 类型仅供 crate 内部使用)
