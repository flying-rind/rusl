# cnd_wait — Rust 接口归约

## 原始 C 接口
```c
int cnd_wait(cnd_t *c, mtx_t *m);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.3)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn cnd_wait(c: *mut cnd_t, m: *mut mtx_t) -> core::ffi::c_int;
```

---

## 意图
原子地释放互斥锁 `m` 并在条件变量 `c` 上阻塞调用线程（无限期等待），直到被 `cnd_signal` 或 `cnd_broadcast` 唤醒。被唤醒后重新获取 `m`。通过调用 `cnd_timedwait(c, m, core::ptr::null())` 实现，利用 musl 内部扩展：将 `NULL` 时间戳指针传给 `cnd_timedwait` 表示无限期等待。

## 前置条件
- `c` 为非空指针，指向通过 `cnd_init` 初始化的条件变量
- `m` 为非空指针，指向通过 `mtx_init` 初始化的互斥锁
- 调用线程必须已锁定 `m`

## 后置条件
- 返回前互斥锁 `m` 已被调用线程重新锁定
- Case 1 被 `cnd_signal` / `cnd_broadcast` 唤醒：返回 `thrd_success` (0)
- Case 2 其他错误：返回 `thrd_error` (2)

## 不变量
- 调用线程在阻塞期间不持有互斥锁 `m`，返回时重新持有

## 算法
直接委托给 `cnd_timedwait`，传入 null 时间戳表示无限等待：

```rust
extern "C" fn cnd_wait(c: *mut cnd_t, m: *mut mtx_t) -> c_int {
    // musl 扩展: NULL 时间戳 => 无限期等待
    cnd_timedwait(c, m, core::ptr::null())
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  cnd_timedwait(c: *mut cnd_t, m: *mut mtx_t, ts: *const timespec) -> c_int
                                      // 依赖1: C11 条件变量定时等待函数（同模块内部转发）
Predefined Macros/Types:
  cnd_t (= pthread_cond_t)             // C11 条件变量类型
  mtx_t (= pthread_mutex_t)            // C11 互斥锁类型
  thrd_success (0) / thrd_error (2)    // C11 返回值枚举

[GUARANTEE]
Exported Interface:
  extern "C" fn cnd_wait(c: *mut cnd_t, m: *mut mtx_t) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 cnd_wait 符号
Internal Interface:
  (同模块内部直接调用 cnd_timedwait，无需额外内部接口)
