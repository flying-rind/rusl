# pthread_setcancelstate — Rust 接口归约

## 原始 C 接口
```c
// 对外导出（POSIX），是 __pthread_setcancelstate 的弱别名
int pthread_setcancelstate(int new, int *old);

// 内部（hidden），pthread_setcancelstate 的主实现
int __pthread_setcancelstate(int new, int *old);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// musl 中 pthread_setcancelstate 是 __pthread_setcancelstate 的弱别名
// Rust 中必须同时导出两者

extern "C" fn pthread_setcancelstate(new: core::ffi::c_int,
    old: *mut core::ffi::c_int) -> core::ffi::c_int;

extern "C" fn __pthread_setcancelstate(new: core::ffi::c_int,
    old: *mut core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
原子地设置调用线程的取消状态（启用或禁用）。当取消被禁用时，来自 `pthread_cancel` 的取消请求将被暂缓（不会丢失），直到取消被重新启用。

## 前置条件
- `new` 取值为 `PTHREAD_CANCEL_ENABLE` (0) 或 `PTHREAD_CANCEL_DISABLE` (1)
- `old` 可为 NULL（不关心旧状态）

## 后置条件
- 返回值 0 表示成功，`self.canceldisable` 被设为 `new`
- 返回值 `EINVAL` 表示 `new` 无效（`new > 2U`）
- 若 `old` 非空，`*old` 被设为先前的取消状态
- 当从禁用切换到启用时，之前挂起的取消请求将在下一个取消点生效

## 不变量
- `canceldisable` 字段仅在值 0（ENABLE）和 1（DISABLE）之间切换
- 取消请求不被丢弃：`cancel` 标志和 `canceldisable` 是独立的

## 算法

```rust
// __pthread_setcancelstate — 主入口
pub extern "C" fn __pthread_setcancelstate(new: core::ffi::c_int,
    old: *mut core::ffi::c_int) -> core::ffi::c_int {
    // 有效性检查：new > 2U（无符号比较）→ 无效
    // 0 和 1 通过，其他值失败
    if (new as core::ffi::c_uint) > 2 {
        return EINVAL;
    }
    // 获取当前线程
    let current = current_thread();
    // 保存旧状态
    if !old.is_null() {
        unsafe { *old = current.canceldisable as core::ffi::c_int; }
    }
    // 设置新状态
    current.canceldisable = new as u8;
    0
}

// pthread_setcancelstate — POSIX 用户接口
pub extern "C" fn pthread_setcancelstate(new: core::ffi::c_int,
    old: *mut core::ffi::c_int) -> core::ffi::c_int {
    __pthread_setcancelstate(new, old)
}
```

对 C 调用者：
1. `extern "C" fn pthread_setcancelstate(new: c_int, old: *mut c_int) -> c_int`
2. 验证 `new` 有效性（0 或 1），无效返回 `EINVAL`
3. 通过 TLS 获取当前线程上下文，原子地读写 `canceldisable` 字段
4. 返回 0 或 `EINVAL`

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
// 使用枚举表示取消状态，避免魔术数字
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CancelState {
    Enable = 0,
    Disable = 1,
}

pub(crate) fn set_cancel_state(new: CancelState) -> CancelState {
    let current = current_thread();
    let old = match current.canceldisable {
        0 => CancelState::Enable,
        _ => CancelState::Disable,
    };
    current.canceldisable = new as u8;
    old
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  __pthread_self() / current_thread()   // 依赖1: 获取当前线程指针（通过 TLS）
Predefined Macros/Constants:
  EINVAL                            // 依赖2: 错误码（来自 <errno.h> 或内部常量）
  PTHREAD_CANCEL_ENABLE (0)        // 依赖3: 取消启用常量
  PTHREAD_CANCEL_DISABLE (1)       // 依赖4: 取消禁用常量

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_setcancelstate(new: core::ffi::c_int, old: *mut core::ffi::c_int) -> core::ffi::c_int;
  extern "C" fn __pthread_setcancelstate(new: core::ffi::c_int, old: *mut core::ffi::c_int) -> core::ffi::c_int;
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_setcancelstate 和 __pthread_setcancelstate 符号
Internal Interface:
  pub(crate) fn set_cancel_state(new: CancelState) -> CancelState;
                                    // 安全包装，供 crate 内部使用
