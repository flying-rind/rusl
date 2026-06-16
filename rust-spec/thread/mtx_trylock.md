# mtx_trylock — Rust 接口归约

## 原始 C 接口
```c
int mtx_trylock(mtx_t *m);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.5)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn mtx_trylock(m: *mut mtx_t) -> core::ffi::c_int;
```

---

## 意图
尝试锁定互斥锁 `m`，但不阻塞。若锁可用则获取并返回成功，若已被占用则立即返回 `thrd_busy`。对普通互斥锁使用快速原子 CAS 路径，对递归互斥锁委托给 `__pthread_mutex_trylock` 内部实现。

## 前置条件
- `m` 为非空指针（`!m.is_null()`），指向通过 `mtx_init` 初始化的互斥锁对象
- 互斥锁未被销毁

## 后置条件
- Case 1 普通互斥锁且锁空闲（`a_cas` 成功）：`_m_lock` 原子设为 `EBUSY`，返回 `thrd_success` (0)，锁被获取
- Case 2 普通互斥锁且已被占用（`a_cas` 返回 `EBUSY`）：返回 `thrd_busy` (1)，锁未获取
- Case 3 非普通互斥锁（递归锁等）：委托 `__pthread_mutex_trylock`，根据返回值映射：
  - `0` -> `thrd_success` (0)
  - `EBUSY` -> `thrd_busy` (1)
  - 其他 -> `thrd_error` (2)

## 不变量
- 获取锁的线程独占互斥锁直至调用 `mtx_unlock`

## 算法
```rust
extern "C" fn mtx_trylock(m: *mut mtx_t) -> c_int {
    unsafe {
        if (*m).m_type() == PTHREAD_MUTEX_NORMAL {
            // 快速路径: a_cas 返回旧值
            // old == 0   => CAS 成功 (锁空闲 -> 设为 EBUSY) => thrd_success
            // old != 0   => CAS 失败 (锁已被占用)       => thrd_busy
            let old = a_cas(&raw mut (*m).m_lock() as *mut c_int, 0, EBUSY);
            return if (old & EBUSY) != 0 { thrd_busy } else { thrd_success };
        }
        // 慢速路径: 递归/检错互斥锁
        let ret = __pthread_mutex_trylock(m as *mut pthread_mutex_t);
        match ret {
            0 => thrd_success,
            EBUSY => thrd_busy,
            _ => thrd_error,
        }
    }
}
```

---

## Rust 内部辅助接口（模块私有）

```rust
// 原子 CAS（同 mtx_lock 模块）
pub(crate) unsafe fn a_cas(p: *mut c_int, t: c_int, s: c_int) -> c_int;

// 内部 POSIX 互斥锁尝试加锁
pub(crate) unsafe fn __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  a_cas(p: *mut c_int, t: c_int, s: c_int) -> c_int  // 依赖1: 原子 CAS
  __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int  // 依赖2: POSIX 内部 trylock
Predefined Macros/Types:
  mtx_t (= pthread_mutex_t)            // C11 互斥锁类型
  PTHREAD_MUTEX_NORMAL (0)             // 普通互斥锁类型
  EBUSY                                // 互斥锁"已锁定"标记
  thrd_success (0) / thrd_busy (1) / thrd_error (2)
  _m_type / _m_lock                    // 互斥锁结构字段访问

[GUARANTEE]
Exported Interface:
  extern "C" fn mtx_trylock(m: *mut mtx_t) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 mtx_trylock 符号
Internal Interface:
  (复用 mtx_lock 模块的 a_cas 和 mtx_timedlock 模块的 __pthread_mutex_trylock)
