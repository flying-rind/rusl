# cnd_broadcast — Rust 接口归约

## 原始 C 接口
```c
int cnd_broadcast(cnd_t *c);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.5)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// cnd_t 在内存布局上等同于 pthread_cond_t，使用 opaque 类型或具体 struct 表示
extern "C" fn cnd_broadcast(c: *mut cnd_t) -> core::ffi::c_int;
```

---

## 意图
唤醒所有在条件变量 `c` 上阻塞等待的线程。若无线程等待，则无效果。是 `__private_cond_signal` 的转发包装器，传递 `-1` 表示"广播全部"。

## 前置条件
- `c` 为非空指针（`!c.is_null()`），指向通过 `cnd_init` 初始化的条件变量对象
- 条件变量未被销毁
- 调用者应当在持有与条件变量关联的互斥锁时调用（非必须但推荐）

## 后置条件
- Case 1 有线程正在 `c` 上等待：所有等待线程被唤醒，每个线程在重新获取关联互斥锁后从 `cnd_wait` 或 `cnd_timedwait` 返回
- Case 2 无线程等待：空操作
- 始终返回 `thrd_success` (0)

## 不变量
- 被唤醒的线程在从等待函数返回前必须重新获取关联的互斥锁

## 算法
直接转发给内部 `__private_cond_signal`，参数 `-1` 表示广播模式：

```rust
extern "C" fn cnd_broadcast(c: *mut cnd_t) -> c_int {
    // unsafe 仅限于 FFI 调用，将 cnd_t* 转换为 pthread_cond_t*
    unsafe { __private_cond_signal(c as *mut pthread_cond_t, -1) }
}
```

内部 `__private_cond_signal` 由 pthread 条件变量模块提供，第二个参数 `n`：正数表示唤醒 n 个等待者，`-1` 表示唤醒全部。

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部条件变量信号函数
pub(crate) unsafe fn __private_cond_signal(c: *mut pthread_cond_t, n: c_int) -> c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __private_cond_signal(c: *mut pthread_cond_t, n: c_int) -> c_int  // 依赖1: musl 内部条件变量信号实现
Predefined Macros/Types:
  cnd_t (= pthread_cond_t 的 typedef)     // C11 条件变量类型
  thrd_success (= 0)                      // C11 成功返回值

[GUARANTEE]
Exported Interface:
  extern "C" fn cnd_broadcast(c: *mut cnd_t) -> core::ffi::c_int;
                                           // 本模块保证对外提供与 C ABI 兼容的 cnd_broadcast 符号
Internal Interface:
  pub(crate) unsafe fn __private_cond_signal(c: *mut pthread_cond_t, n: c_int) -> c_int;
                                           // 内部条件变量信号，模块间共享
