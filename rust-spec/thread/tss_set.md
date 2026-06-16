# tss_set — Rust 接口归约

## 原始 C 接口
```c
int tss_set(tss_t k, void *x);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.6.4)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// tss_t = c_uint，值传递
extern "C" fn tss_set(
    k: core::ffi::c_uint,
    x: *mut core::ffi::c_void,
) -> core::ffi::c_int;
```

---

## 意图
在线程特定存储中为当前线程设置键 `k` 关联的值为 `x`。直接访问内部线程控制块的 `tsd[]` 数组实现高性能存取。包含写时复制 (COW) 优化：若新值与旧值相同则跳过写入，避免在 fork 后触发不必要的内存复制。

## 前置条件
- `k` 是通过 `tss_create` 创建的有效 TSS 键
- 在可通过 `__pthread_self()` 获取内部线程结构的上下文中调用（非信号处理器上下文）

## 后置条件
- `self->tsd[k] = x`（仅当旧值不等于 `x` 时执行写入）
- 若执行了写入：`self->tsd_used = 1`，标记该线程使用了 TSD（触发线程退出时的析构函数扫描）
- 始终返回 `thrd_success` (0)

## 不变量
- `self->tsd_used == 1` 当且仅当至少一个 TSS 键被设为了非 NULL 值
- COW 优化依赖于 fork 子进程只读复制父进程内存页的机制

## 算法
```rust
extern "C" fn tss_set(k: c_uint, x: *mut c_void) -> c_int {
    unsafe {
        let self_ptr = __pthread_self(); // 获取当前线程的 struct pthread *
        let tsd_slot = &raw mut (*self_ptr).tsd[k as usize];
        // COW 优化: 仅当值改变时才写入，避免 fork 后不必要的页复制
        if *tsd_slot != x {
            *tsd_slot = x;
            (*self_ptr).tsd_used = 1;    // 标记此线程有 TSD 需要析构
        }
    }
    thrd_success
}
```

`struct pthread` 是 musl 线程控制块，包含 `tsd: *mut *mut c_void`（TSD 数组指针）和 `tsd_used: u8`（位域标记）。这些内部结构以 `repr(C)` 布局与 C 完全兼容。

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部线程控制块（repr(C) 与 C struct pthread 布局兼容）
#[repr(C)]
pub(crate) struct Pthread {
    // ... 其他字段 ...
    pub(crate) tsd: *mut *mut c_void,  // TSD 数组指针
    pub(crate) tsd_used: u8,           // TSD 使用标记位域
    // ... 其他字段 ...
}

// 获取当前线程控制块
pub(crate) unsafe fn __pthread_self() -> *mut Pthread;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_self() -> *mut Pthread     // 依赖1: 获取当前线程控制块
  struct pthread { tsd, tsd_used, ... } // 依赖2: musl 线程控制块布局 (repr(C))
Predefined Macros/Types:
  tss_t (= c_uint)                     // C11 TSS 键类型（用作 TSD 数组索引）
  thrd_success (= 0)                   // C11 成功返回值

[GUARANTEE]
Exported Interface:
  extern "C" fn tss_set(k: core::ffi::c_uint, x: *mut core::ffi::c_void) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 tss_set 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_self() -> *mut Pthread;
                                        // 内部获取当前线程，模块间共享
