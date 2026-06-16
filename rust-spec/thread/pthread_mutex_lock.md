# pthread_mutex_lock — Rust 接口归约

## 原始 C 接口
```c
int __pthread_mutex_lock(pthread_mutex_t *m);
int pthread_mutex_lock(pthread_mutex_t *m);  // weak_alias → __pthread_mutex_lock
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_mutex_lock 是主实现，pthread_mutex_lock 是其 weak_alias
// rusl 必须同时导出两者
extern "C" fn __pthread_mutex_lock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
extern "C" fn pthread_mutex_lock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
```

---

## 意图
以阻塞方式获取互斥锁。若互斥锁已被其他线程持有，调用线程将阻塞直到互斥锁可用。NORMAL 类型通过快速 CAS 路径尝试立即获取，失败后委托给 `__pthread_mutex_timedlock` 完成阻塞等待。

## 前置条件
- `m` 非空指针（`!m.is_null()`）
- `m` 指向一个已初始化的 `pthread_mutex_t`

## 后置条件
- Case 1 成功获取：
  - 互斥锁被调用线程锁定
  - 若为 RECURSIVE 类型且已由调用线程持有：`_m_count` 递增
  - 返回值为 `0`
- Case 2 死锁检测（ERRORCHECK 类型重复加锁）：
  - 返回值为 `EDEADLK`
- Case 3 互斥锁处于 EOWNERDEAD 状态（robust）：
  - 互斥锁被获取，返回值为 `EOWNERDEAD`
- Case 4 被信号中断：
  - 可能被重启，取决于具体实现路径

## 不变量
- 加锁成功后 `_m_lock` 记录了当前线程的 tid

## 算法
两阶段策略：

```rust
// 两个导出符号委托到同一内部实现
extern "C" fn pthread_mutex_lock(m: *mut pthread_mutex_t) -> core::ffi::c_int {
    __pthread_mutex_lock(m)
}

extern "C" fn __pthread_mutex_lock(m: *mut pthread_mutex_t) -> core::ffi::c_int {
    unsafe {
        let mref = &*m;
        let mtype = mref.get_type() & 15;
        // 阶段 1: NORMAL 类型快速路径 — CAS 尝试立即获取
        if mtype == MUTEX_TYPE_NORMAL {
            if atomic_cas(&mref.lock(), 0, EBUSY) == 0 {
                return 0; // 无竞争，立即获取
            }
        }
        // 阶段 2: 通用路径 — 委托给 timedlock (超时参数为 None = 无限等待)
        __pthread_mutex_timedlock(m, core::ptr::null())
    }
}
```

Rust 内部实现中，快速 CAS 路径使用 `core::sync::atomic::AtomicI32::compare_exchange`，通用路径委托给 `timedlock` 且超时参数为 `None`。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
impl PthreadMutex {
    pub(crate) fn lock(&self) -> Result<(), core::ffi::c_int> {
        // 安全包装：阻塞加锁
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  atomic_cas (内部模块)             // 依赖: 原子比较交换，NORMAL 快速路径
  __pthread_mutex_timedlock (本模块) // 依赖: 带超时的通用加锁，timeout=NULL 表示无限等待
  core::sync::atomic::Ordering     // 依赖: 原子操作内存排序

[GUARANTEE]
Exported Interface:
  extern "C" fn __pthread_mutex_lock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
                                 // musl 内部实现符号，rusl 必须导出
  extern "C" fn pthread_mutex_lock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_lock 符号
Internal Interface:
  impl PthreadMutex::lock(&self) -> Result<(), core::ffi::c_int>;
                                 // 安全包装，供 crate 内部使用
