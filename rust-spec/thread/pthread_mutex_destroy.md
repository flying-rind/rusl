# pthread_mutex_destroy — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutex_destroy(pthread_mutex_t *mutex);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> core::ffi::c_int;
```

---

## 意图
销毁互斥锁对象。若互斥锁为进程共享且跟踪所有权（`_m_type > 128` 意味着至少设置了 process-shared 位），则在销毁前需等待虚拟内存区域的静默期（quiescence），确保互斥锁不再被其他进程的 robust list 引用。

## 前置条件
- `mutex` 非空指针（`!mutex.is_null()`）
- `mutex` 指向一个已初始化的 `pthread_mutex_t`
- 互斥锁应处于未加锁状态（POSIX 要求，musl 不强制检查）
- 销毁后不能再使用该互斥锁

## 后置条件
- Case 1（总是成功）：
  - 若 `(*mutex)._m_type > 128`：调用内部 `vm_wait()` 等待 robust list 静默
  - 返回值为 `0`

## 不变量
- 调用 `vm_wait()` 后，互斥锁不再被任何 robust_list pending 槽引用

## 算法

```rust
extern "C" fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> core::ffi::c_int {
    unsafe {
        if (*mutex).get_type() > 128 {
            // 进程共享互斥锁：等待 pending 操作完成
            vm_wait();
        }
    }
    0
}
```

内部 Rust 实现中，进程共享互斥锁的 robust list 保护使用 `vm_lock()`/`vm_unlock()` 机制。`vm_wait()` 确保所有正在进行的 VM 区域操作完成后才返回。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
pub(crate) fn mutex_destroy(mutex: &mut PthreadMutex) {
    // 若为进程共享类型，调用 vm_wait() 确保静默
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  vm_wait() (内部模块)             // 依赖: 等待虚拟内存区域静默期

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_destroy 符号
Internal Interface:
  pub(crate) fn mutex_destroy(mutex: &mut PthreadMutex);
                                 // 安全包装，供 crate 内部使用
