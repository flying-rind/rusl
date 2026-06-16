# mtx_unlock — Rust 接口归约

## 原始 C 接口
```c
int mtx_unlock(mtx_t *mtx);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.6)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn mtx_unlock(mtx: *mut mtx_t) -> core::ffi::c_int;
```

---

## 意图
解锁由调用线程持有的互斥锁。若存在等待该互斥锁的线程，则其中一个被唤醒以获取锁。是 `__pthread_mutex_unlock` 的直接转发调用。

## 前置条件
- `mtx` 为非空指针（`!mtx.is_null()`），指向通过 `mtx_init` 初始化的互斥锁对象
- 调用线程必须持有该互斥锁（`mtx_lock` / `mtx_trylock` 先前成功），否则行为未定义
- 递归互斥锁需匹配 `mtx_lock` 调用次数

## 后置条件
- 互斥锁被释放。若有等待者则其中一个被唤醒
- 返回值透传自 `__pthread_mutex_unlock`。对于 C11 语义，`pthread_mutex_unlock` 返回错误的情况皆属未定义行为（如解锁非持有的锁），故假设始终成功

## 不变量
- 解锁后调用线程不再持有该互斥锁

## 算法
直接尾调用转发给内部 POSIX 实现：

```rust
extern "C" fn mtx_unlock(mtx: *mut mtx_t) -> c_int {
    unsafe {
        __pthread_mutex_unlock(mtx as *mut pthread_mutex_t)
    }
}
```

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 POSIX 互斥锁解锁
pub(crate) unsafe fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int  // 依赖1: musl 内部互斥锁解锁实现
Predefined Macros/Types:
  mtx_t (= pthread_mutex_t, repr(C))  // C11 互斥锁类型

[GUARANTEE]
Exported Interface:
  extern "C" fn mtx_unlock(mtx: *mut mtx_t) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 mtx_unlock 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;
                                        // 内部互斥锁解锁，模块间共享
