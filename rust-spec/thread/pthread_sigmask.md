# pthread_sigmask — Rust 接口归约

## 原始 C 接口
```c
int pthread_sigmask(int how, const sigset_t *restrict set, sigset_t *restrict old);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// sigset_t 需保持与 musl 的 ABI 兼容的内存布局
// musl 中 sigset_t 包含 __bits 数组（32位系统为 32×2=64 位，64位系统为 64 位）

extern "C" fn pthread_sigmask(how: core::ffi::c_int,
    set: *const Sigset, old: *mut Sigset) -> core::ffi::c_int;
```

---

## 意图
检查和/或修改调用线程的信号掩码。与 `sigprocmask()` 的功能相同，但在多线程环境中 POSIX 要求使用此函数。从 `old` 返回值中清除内部信号位（`SIGCANCEL`、`SIGSYNCCALL`），避免用户观察到 libc 内部使用的信号。

## 前置条件
- `how` 为 `SIG_BLOCK` (0)、`SIG_UNBLOCK` (1) 或 `SIG_SETMASK` (2)
- `set` 可为 NULL（仅查询当前掩码时）
- `old` 可为 NULL（不关心旧掩码时）
- 若 `set` 非空，`how` 必须有效

## 后置条件
- Case 1（`set != NULL` 且 `how` 无效）：返回 `EINVAL`
- Case 2（有效调用）：通过 `SYS_rt_sigprocmask` 系统调用修改线程信号掩码
  - 若系统调用成功（返回 0）且 `old` 非空：`*old` 包含之前的信号掩码，但内部信号位被清除
  - 清除的信号位：64 位系统上清除 `__bits[0]` 的位 32-33；32 位系统上清除 `__bits[0]` 的位 31 和 `__bits[1]` 的位 0-1
  - 返回值为系统调用的错误码（0 = 成功）

## 不变量
- 线程实际信号掩码包含内部信号（用于线程取消和同步调用），但 `old` 返回值中这些位被清除，避免用户误操作
- 信号处理函数中通过 `uc_sigmask` 获取运行上下文时，内部信号位也会被妥善屏蔽

## 算法

```rust
// Sigset 的 ABI 兼容表示
// musl 中 sigset_t 包含 unsigned long __bits[128/sizeof(long)]
// 64 位系统: __bits: [u64; 2] (128 bits)
// 32 位系统: __bits: [u32; 4] (128 bits)
#[repr(C)]
pub struct Sigset {
    __bits: [usize; _NSIG_WORDS],  // _NSIG_WORDS = (128 + 8*sizeof(usize) - 1) / (8*sizeof(usize))
}

// pthread_sigmask — 对外导出函数
pub extern "C" fn pthread_sigmask(how: core::ffi::c_int,
    set: *const Sigset, old: *mut Sigset) -> core::ffi::c_int {

    // 1. 参数有效性检查
    if !set.is_null() && (how as core::ffi::c_uint) - SIG_BLOCK as core::ffi::c_uint > 2 {
        return EINVAL;
    }

    // 2. 系统调用
    let sigset_size = _NSIG / 8;  // musl 中为 128/8 = 16 或等效值
    let ret = unsafe {
        __syscall(SYS_rt_sigprocmask, how, set, old, sigset_size)
    };

    // 3. 清除内部信号位
    if ret == 0 && !old.is_null() {
        unsafe {
            clear_internal_sigs(&mut *old);
        }
    }

    if ret < 0 { (-ret) as core::ffi::c_int } else { 0 }
}

// 清除内部信号位（SIGCANCEL=33, SIGSYNCCALL=34）
fn clear_internal_sigs(set: &mut Sigset) {
    // 64 位系统: __bits[0] &= ~0x380000000ULL（清除位 32-33）
    // 32 位系统: __bits[0] &= ~0x80000000UL; __bits[1] &= ~0x3UL
    if core::mem::size_of::<usize>() == 8 {
        set.__bits[0] &= !0x380000000;
    } else {
        set.__bits[0] &= !0x80000000u32 as usize;
        set.__bits[1] &= !0x3;
    }
}
```

对 C 调用者：
1. `extern "C" fn pthread_sigmask(how: c_int, set: *const Sigset, old: *mut Sigset) -> c_int`
2. 内部调用 `SYS_rt_sigprocmask` 并清除内部信号位
3. 返回 0 或 errno

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
// 使用枚举避免魔术数字

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SigmaskHow {
    Block = 0,    // SIG_BLOCK
    Unblock = 1,  // SIG_UNBLOCK
    Setmask = 2,  // SIG_SETMASK
}

pub(crate) fn thread_sigmask(how: SigmaskHow, set: Option<&Sigset>, old: Option<&mut Sigset>)
    -> Result<(), Errno>
{
    let how_int = how as core::ffi::c_int;
    let set_ptr = set.map_or(core::ptr::null(), |s| s as *const Sigset);
    let old_ptr = old.map_or(core::ptr::null_mut(), |o| o as *mut Sigset);
    let ret = pthread_sigmask(how_int, set_ptr, old_ptr);
    if ret == 0 { Ok(()) } else { Err(Errno::from(ret)) }
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  __syscall(SYS_rt_sigprocmask, ...)    // 依赖1: rt_sigprocmask 系统调用
Predefined Macros/Constants:
  SIG_BLOCK (0)                         // 依赖2
  SIG_UNBLOCK (1)                       // 依赖3
  SIG_SETMASK (2)                       // 依赖4
  EINVAL                                // 依赖5: 错误码
  _NSIG                                 // 依赖6: 系统信号总数常量
  SYS_rt_sigprocmask                    // 依赖7: 系统调用号
Predefined Structures:
  Sigset (sigset_t)                     // 依赖8: 信号集类型（repr(C) ABI 兼容）
Internal Signals:
  SIGCANCEL (33)                        // 依赖9: 线程取消内部信号
  SIGSYNCCALL (34)                      // 依赖10: 同步调用内部信号

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_sigmask(how: core::ffi::c_int, set: *const Sigset, old: *mut Sigset)
      -> core::ffi::c_int;
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_sigmask 符号
Internal Interface:
  pub(crate) fn thread_sigmask(how: SigmaskHow, set: Option<&Sigset>, old: Option<&mut Sigset>)
      -> Result<(), Errno>;
                                    // 安全包装，供 crate 内部使用
