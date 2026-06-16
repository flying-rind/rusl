# thrd_create — Rust 接口归约

## 原始 C 接口
```c
int thrd_create(thrd_t *thr, thrd_start_t func, void *arg);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.1)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// thrd_t 在 C 中为 struct __pthread *，在 Rust 中表示为 opaque 指针
// thrd_start_t = extern "C" fn(*mut c_void) -> c_int
extern "C" fn thrd_create(
    thr: *mut thrd_t,
    func: Option<extern "C" fn(arg: *mut core::ffi::c_void) -> core::ffi::c_int>,
    arg: *mut core::ffi::c_void,
) -> core::ffi::c_int;
```

---

## 意图
创建一个新的执行线程，执行 `func(arg)`，并将线程标识符存入 `*thr`。是 `__pthread_create` 的包装器，传递特殊属性 `__ATTRP_C11_THREAD` 以指示这是 C11 线程（区别于 POSIX 线程），并负责 C 类型转换与返回值映射。

## 前置条件
- `thr` 为非空指针（`!thr.is_null()`），指向有效的 `thrd_t` 内存位置
- `func` 不为 `None`，签名 `extern "C" fn(*mut c_void) -> c_int` 的有效函数指针
- `arg` 可为任意指针值
- 系统资源足够创建新线程

## 后置条件
- Case 1 成功：`*thr` 存储新线程标识符，线程开始执行 `func(arg)`，返回 `thrd_success` (0)
- Case 2 内存不足（`EAGAIN`）：线程无法创建，返回 `thrd_nomem` (3)
- Case 3 其他错误：返回 `thrd_error` (2)

## 不变量
- 新线程与创建线程并发执行，共享进程地址空间
- C11 线程由内部属性 `__ATTRP_C11_THREAD` (= `(void*)(uintptr_t)-1`) 标记，用于 exit/join 语义区分

## 算法
```rust
extern "C" fn thrd_create(
    thr: *mut thrd_t,
    func: Option<extern "C" fn(*mut c_void) -> c_int>,
    arg: *mut c_void,
) -> c_int {
    // C11 thrd_start_t (int (*)(void*)) 与 POSIX start_routine (void* (*)(void*)) 类型不同
    // 但 ABI 兼容，直接强制转换函数指针
    let ret = unsafe {
        __pthread_create(
            thr,
            __ATTRP_C11_THREAD,                          // 标记为 C11 线程
            func.map(|f| f as extern "C" fn(*mut c_void) -> *mut c_void), // ABI 兼容的类型强转
            arg,
        )
    };
    match ret {
        0 => thrd_success,
        EAGAIN => thrd_nomem,
        _ => thrd_error,
    }
}
```

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 POSIX 线程创建
pub(crate) unsafe fn __pthread_create(
    thr: *mut thrd_t,
    attr: *const c_void,                                    // __ATTRP_C11_THREAD 或 null
    start: Option<extern "C" fn(*mut c_void) -> *mut c_void>,
    arg: *mut c_void,
) -> c_int;

// C11 线程属性标记
pub(crate) const __ATTRP_C11_THREAD: *const c_void = usize::MAX as *const c_void; // (void*)(uintptr_t)-1
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_create(thr, attr, start, arg) -> c_int  // 依赖1: musl 内部线程创建实现
Predefined Macros/Types:
  thrd_t (= struct __pthread *)        // C11 线程标识符类型（repr(C) opaque）
  thrd_start_t = extern "C" fn(*mut c_void) -> c_int
  __ATTRP_C11_THREAD (= (void*)(uintptr_t)-1)  // C11 线程属性标记
  thrd_success (0) / thrd_nomem (3) / thrd_error (2)
  EAGAIN                               // 资源暂时不可用 errno

[GUARANTEE]
Exported Interface:
  extern "C" fn thrd_create(
      thr: *mut thrd_t,
      func: Option<extern "C" fn(*mut c_void) -> c_int>,
      arg: *mut c_void,
  ) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 thrd_create 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_create(
      thr: *mut thrd_t,
      attr: *const c_void,
      start: Option<extern "C" fn(*mut c_void) -> *mut c_void>,
      arg: *mut c_void,
  ) -> c_int;
                                        // 内部线程创建，模块间共享
