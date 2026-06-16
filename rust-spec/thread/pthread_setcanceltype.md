# pthread_setcanceltype — Rust 接口归约

## 原始 C 接口
```c
int pthread_setcanceltype(int new, int *old);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// 注意：musl 中此函数无 __ 前缀弱别名，直接为对外导出符号

extern "C" fn pthread_setcanceltype(new: core::ffi::c_int,
    old: *mut core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
设置调用线程的取消类型。延迟取消（默认，`PTHREAD_CANCEL_DEFERRED`）仅在取消点进行检查；异步取消（`PTHREAD_CANCEL_ASYNCHRONOUS`）可在任意时刻生效。切换到异步取消时，若有挂起的取消请求则立即生效。

## 前置条件
- `new` 必须为 `PTHREAD_CANCEL_DEFERRED` (0) 或 `PTHREAD_CANCEL_ASYNCHRONOUS` (1)
- `old` 可为 NULL

## 后置条件
- Case 1（`new > 1U`，无效值）：返回 `EINVAL`，`self.cancelasync` 不变
- Case 2（有效值）：若 `old` 非空，`*old = self.cancelasync`。`self.cancelasync = new`。若 `new` 为非零值（切换到异步），调用 `pthread_testcancel()` 立即检查是否有挂起的取消请求。返回 0。

## 不变量
- `cancelasync` 字段仅在值 0（DEFERRED）和 1（ASYNCHRONOUS）之间切换
- 取消类型不影响取消请求的存在性，仅影响生效时机

## 算法

```rust
// pthread_setcanceltype — 对外导出函数
pub extern "C" fn pthread_setcanceltype(new: core::ffi::c_int,
    old: *mut core::ffi::c_int) -> core::ffi::c_int {
    // 有效性检查：new > 1U（无符号比较）→ 无效
    // 0 (DEFERRED) 通过，1 (ASYNCHRONOUS) 通过，其他值失败
    if (new as core::ffi::c_uint) > 1 {
        return EINVAL;
    }
    // 获取当前线程
    let current = current_thread();
    // 保存旧类型
    if !old.is_null() {
        unsafe { *old = current.cancelasync as core::ffi::c_int; }
    }
    // 设置新类型
    current.cancelasync = new as u8;
    // 切换到异步取消时立即检查是否有挂起的取消请求
    if new != 0 {
        pthread_testcancel();
    }
    0
}
```

对 C 调用者：
1. `extern "C" fn pthread_setcanceltype(new: c_int, old: *mut c_int) -> c_int`
2. 验证 `new` 有效性，无效返回 `EINVAL`
3. 通过 TLS 获取当前线程上下文，读写 `cancelasync` 字段
4. 若切换到异步取消模式，调用 `pthread_testcancel()` 立即检查
5. 返回 0 或 `EINVAL`

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
// 使用枚举避免魔术数字
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CancelType {
    Deferred = 0,
    Asynchronous = 1,
}

pub(crate) fn set_cancel_type(new: CancelType) -> CancelType {
    let current = current_thread();
    let old = match current.cancelasync {
        0 => CancelType::Deferred,
        _ => CancelType::Asynchronous,
    };
    current.cancelasync = new as u8;
    if new == CancelType::Asynchronous {
        testcancel();  // 立即检查挂起的取消请求
    }
    old
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  __pthread_self() / current_thread()   // 依赖1: 获取当前线程指针（通过 TLS）
  pthread_testcancel()                  // 依赖2: 取消点检查（定义在 pthread_testcancel 模块）
Predefined Macros/Constants:
  EINVAL                            // 依赖3: 错误码
  PTHREAD_CANCEL_DEFERRED (0)      // 依赖4: 延迟取消常量
  PTHREAD_CANCEL_ASYNCHRONOUS (1)  // 依赖5: 异步取消常量

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_setcanceltype(new: core::ffi::c_int, old: *mut core::ffi::c_int) -> core::ffi::c_int;
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_setcanceltype 符号
Internal Interface:
  pub(crate) fn set_cancel_type(new: CancelType) -> CancelType;
                                    // 安全包装，供 crate 内部使用
