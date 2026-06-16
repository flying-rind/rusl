# thrd_exit — Rust 接口归约

## 原始 C 接口
```c
_Noreturn void thrd_exit(int result);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.2)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// Rust 中用 -> ! 表示永不返回（对应 C 的 _Noreturn）
extern "C" fn thrd_exit(result: core::ffi::c_int) -> !;
```

---

## 意图
终止调用线程，将 `result` 设定为该线程的退出码。通过 `thrd_join` 获取此退出码。由于 POSIX 线程退出码为 `void *`，需将 `c_int` 通过 `isize` 转换为 `*mut c_void`。函数永不返回。

## 前置条件
- `result` 为任意整型值（线程退出码）

## 后置条件
- 调用线程终止，不再执行后续代码
- 线程的退出码可通过 `thrd_join` 以 `c_int` 形式获取
- 若存在 TSS 析构函数，按实现定义的行为执行
- 函数永不返回

## 不变量
- 函数签名中 `-> !`（never 类型）确保 Rust 编译器知晓控制流终止

## 算法
```rust
extern "C" fn thrd_exit(result: c_int) -> ! {
    // 将 c_int 通过 intptr_t 转换为 void*（保持值语义）
    unsafe {
        __pthread_exit(result as isize as *mut c_void);
    }
    // unreachable: __pthread_exit 永不返回
    unreachable!();
}
```

注意：C 中 `intptr_t` 往返转换保证 `int -> void* -> int` 无损。Rust 中 `c_int` 为 `i32`，`isize` 在 64 位系统上为 `i64`，转换路径 `c_int -> isize -> *mut c_void -> isize -> c_int` 确保值不变。

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 POSIX 线程退出
pub(crate) unsafe fn __pthread_exit(retval: *mut c_void) -> !;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_exit(retval: *mut c_void) -> !  // 依赖1: musl 内部线程退出实现
Predefined Macros/Types:
  core::ffi::c_int (= i32)             // C int 类型
  core::ffi::c_void                    // C void 类型
  isize                                // 指针宽度有符号整数 (等价于 C 的 intptr_t)

[GUARANTEE]
Exported Interface:
  extern "C" fn thrd_exit(result: core::ffi::c_int) -> !;
                                        // 本模块保证对外提供与 C ABI 兼容的 thrd_exit 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_exit(retval: *mut c_void) -> !;
                                        // 内部线程退出，模块间共享
