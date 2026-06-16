# thrd_join — Rust 接口归约

## 原始 C 接口
```c
int thrd_join(thrd_t t, int *res);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.4)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn thrd_join(t: thrd_t, res: *mut core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
阻塞调用线程直到线程 `t` 终止，并可选地获取其退出码。是 `__pthread_join` 的包装器，将 POSIX 的 `void *` 退出码通过 `isize` 转换为 `c_int` 存入 `*res`。

## 前置条件
- `t` 是一个有效的、可 join 的线程标识符（未被 `thrd_detach` 分离）
- 尚未有其他线程在对 `t` 调用 `thrd_join`
- `res` 可为 `null`（不关心退出码）或指向有效 `c_int` 的指针

## 后置条件
- 调用线程阻塞直到线程 `t` 终止
- 若 `res` 非空：`*res = pthread_res as c_int`，即线程 `t` 传递给 `thrd_exit` 的值或 `func` 的返回值
- 线程 `t` 的资源被回收，不再可 join
- 始终返回 `thrd_success` (0)

## 不变量
- 每个可 join 线程恰好被 join 一次（或 detach 一次）
- 经 `isize` 往返转换：`thrd_exit(v)` -> `thrd_join` -> `v`（无损）

## 算法
```rust
extern "C" fn thrd_join(t: thrd_t, res: *mut c_int) -> c_int {
    let mut pthread_res: *mut c_void = core::ptr::null_mut();
    unsafe {
        __pthread_join(t, &raw mut pthread_res);
    }
    if !res.is_null() {
        unsafe {
            // 将 void* 退出码通过 isize 转回 c_int
            *res = pthread_res as isize as c_int;
        }
    }
    thrd_success
}
```

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 POSIX 线程 join
pub(crate) unsafe fn __pthread_join(t: thrd_t, retval: *mut *mut c_void) -> c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_join(t: thrd_t, retval: *mut *mut c_void) -> c_int  // 依赖1: musl 内部线程 join 实现
Predefined Macros/Types:
  thrd_t (= struct __pthread *)        // C11 线程标识符类型
  core::ffi::c_int (= i32)             // C int 类型
  thrd_success (= 0)                   // C11 成功返回值
  isize                                // 指针宽度有符号整数 (等价于 C 的 intptr_t)

[GUARANTEE]
Exported Interface:
  extern "C" fn thrd_join(t: thrd_t, res: *mut core::ffi::c_int) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 thrd_join 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_join(t: thrd_t, retval: *mut *mut c_void) -> c_int;
                                        // 内部线程 join，模块间共享
