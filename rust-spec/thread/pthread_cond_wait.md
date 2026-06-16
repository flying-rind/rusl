# pthread_cond_wait -- Rust 接口归约

## 原始 C 接口
```c
int pthread_cond_wait(pthread_cond_t *restrict c, pthread_mutex_t *restrict m);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_cond_wait(
    c: *mut pthread_cond_t,
    m: *mut pthread_mutex_t,
) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
在条件变量上无限期阻塞当前线程，原子释放 mutex 并进入等待，直至被 signal/broadcast 唤醒。等价于 `pthread_cond_timedwait(c, m, NULL)`。

## 前置条件
- `c` 非空，指向有效 `pthread_cond_t`
- `m` 非空，指向已由当前线程锁定的 `pthread_mutex_t`

## 后置条件
- 返回时 mutex 已由当前线程重新锁定
- Case 1 正常被唤醒：返回 `0`
- Case 2 被取消：返回 `ECANCELED`（受 __pthread_cond_timedwait 取消规则约束）
- Case 3 被信号中断：返回 `EINTR`
- Case 4 mutex 校验失败：返回 `EPERM`

## 不变量
无。本函数纯粹作为转发代理。

## 算法
```rust
pub extern "C" fn pthread_cond_wait(
    c: *mut pthread_cond_t,
    m: *mut pthread_mutex_t,
) -> c_int {
    // ts = NULL 表示无限等待
    pthread_cond_timedwait(c, m, core::ptr::null())
}
```

## Rust 内部设计要点
- 完全转发给 `pthread_cond_timedwait`，`ts = null()` 表示无超时
- 函数签名保持 ABI 兼容，所有参数类型与 C 一致
- 内部实现为一个简单的尾调用，无额外开销

---

/* Rely */
[RELY]
Predefined Types:
  pthread_cond_t                   // #[repr(C)] 条件变量类型
  pthread_mutex_t                  // #[repr(C)] 互斥锁类型
  core::ffi::c_int                // Rust 核心库 C FFI 类型

External Functions (同一 crate):
  pthread_cond_timedwait           // 条件变量超时等待实现，ts=NULL 表示无限等待

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_cond_wait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
