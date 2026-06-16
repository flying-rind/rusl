# pthread_mutex_unlock — Rust 接口归约

## 原始 C 接口
```c
int __pthread_mutex_unlock(pthread_mutex_t *m);
int pthread_mutex_unlock(pthread_mutex_t *m);  // weak_alias → __pthread_mutex_unlock
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_mutex_unlock 是主实现，pthread_mutex_unlock 是其 weak_alias
// rusl 必须同时导出两者
extern "C" fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
extern "C" fn pthread_mutex_unlock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
```

---

## 意图
释放调用线程持有的互斥锁。处理递归计数、robust list 摘除、PI futex 解锁以及唤醒等待者。NORMAL 类型仅做锁释放和唤醒，不维护 robust list。

## 前置条件
- `m` 非空指针（`!m.is_null()`）
- `m` 指向一个已初始化的 `pthread_mutex_t`
- 非 NORMAL 类型时：调用线程必须是锁的持有者

## 后置条件
- Case 1 NORMAL 类型：
  - `_m_lock` 被设置为 0（或 new 值）
  - 若存在等待者或 `cont < 0`：调用 `__wake` 唤醒一个等待者
  - 返回值为 `0`
- Case 2 RECURSIVE 类型且 `_m_count > 0`：
  - `_m_count` 递减 1，互斥锁仍被持有
  - 返回值为 `0`
- Case 3 非 NORMAL 类型且调用者非持有者：
  - 返回值为 `EPERM`
- Case 4 ROBUST 类型且原锁处于 EOWNERDEAD：
  - `new = 0x7fffffff`（标记为不可恢复）
  - 释放锁后存在等待者时唤醒
  - 返回值为 `0`
- Case 5 PI 类型（type & 8）：
  - CAS 尝试设置为 new，失败时通过 `FUTEX_UNLOCK_PI` futex 解锁
  - 返回值为 `0`

## 不变量
- 解锁后互斥锁不再存在于线程的 `robust_list` 链表中
- NORMAL 类型不维护 robust list，因此跳过所有权校验和链表操作
- `vm_lock()` 和 `vm_unlock()` 成对调用，保护 robust_list 链表操作的原子性
- `robust_list.pending` 在函数返回前必然被清零

## 算法

```rust
extern "C" fn pthread_mutex_unlock(m: *mut pthread_mutex_t) -> core::ffi::c_int {
    __pthread_mutex_unlock(m)
}

extern "C" fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> core::ffi::c_int {
    unsafe {
        let mref = &*m;
        let waiters = mref.get_waiters();
        let mtype = mref.get_type() & 15;
        let priv_flag = (mref.get_type() & 128) ^ 128;
        let mut new = 0;

        if mtype != MUTEX_TYPE_NORMAL {
            let self_tid = current_tid();
            let old = mref.get_lock();
            let own = old & 0x3fffffff;

            // 所有权校验
            if own != self_tid {
                return libc_errcode::EPERM;
            }

            // 递归计数递减
            if (mtype & 3) == MUTEX_TYPE_RECURSIVE && mref.get_count() > 0 {
                mref.dec_count();
                return 0;
            }

            // robust deadlock → ENOTRECOVERABLE 标记
            if (mtype & 4) != 0 && (old & 0x40000000) != 0 {
                new = 0x7fffffff;
            }

            // 进程私有锁：VM 锁保护 robust_list 链表操作
            if priv_flag == FUTEX_PRIVATE {
                current_set_pending(&mref.next_ptr());
                vm_lock();
            }

            // 从 robust_list 链表中摘除
            unlink_from_robust_list(mref);

            // 解锁
            if (mtype & 8) != 0 {
                // PI 类型：CAS 或 PI futex 解锁
                if old < 0 || atomic_cas(&mref.lock(), old, new) != old {
                    if new != 0 {
                        mref.set_waiters(-1);
                    }
                    futex_unlock_pi(&mref.lock(), priv_flag);
                }
            } else {
                // 普通类型：原子交换
                let cont = atomic_swap(&mref.lock(), new);
            }

            // 清理
            if priv_flag == FUTEX_PRIVATE {
                current_clear_pending();
                vm_unlock();
            }
        }

        // 唤醒等待者
        if waiters != 0 || /* cont < 0 */ {
            futex_wake(&mref.lock(), 1, priv_flag);
        }

        0
    }
}
```

内部 Rust 实现中：
- `vm_lock()`/`vm_unlock()` 提供 robust_list 链表操作的临界区保护
- `unlink_from_robust_list()` 执行 `prev.next = next; next.prev = prev` 的标准双链表摘除
- PI futex 使用 `linux_futex` 模块的 `FUTEX_UNLOCK_PI` 操作
- 原子操作用 `core::sync::atomic` 实现

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
impl PthreadMutex {
    pub(crate) fn unlock(&self) -> Result<(), core::ffi::c_int> {
        // 安全包装：解锁
        // 返回 Err(EPERM) 表示非持有者调用
    }
}
```

---

## 与 trylock 的协作关系

unlock 中的 robust_list 链表摘除操作对应 trylock 中的链表插入操作。两者配合维护 per-thread robust list 的正确性。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  atomic_cas / atomic_swap (内部模块)   // 依赖: 原子比较交换和交换操作
  current_thread / current_tid (内部模块) // 依赖: 获取当前线程控制块和 tid
  linux_futex (内部模块)                // 依赖: PI futex 解锁（FUTEX_UNLOCK_PI）和唤醒（FUTEX_WAKE）
  vm_lock / vm_unlock (内部模块)       // 依赖: 虚拟内存区域锁，保护 robust list
  core::sync::atomic::Ordering          // 依赖: 原子操作内存排序

[GUARANTEE]
Exported Interface:
  extern "C" fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
                                 // musl 内部实现符号，rusl 必须导出
  extern "C" fn pthread_mutex_unlock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_unlock 符号
Internal Interface:
  impl PthreadMutex::unlock(&self) -> Result<(), core::ffi::c_int>;
                                 // 安全包装，供 crate 内部使用
