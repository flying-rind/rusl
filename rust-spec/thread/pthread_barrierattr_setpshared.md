# pthread_barrierattr_setpshared -- Rust 接口归约

## 原始 C 接口
```c
int pthread_barrierattr_setpshared(pthread_barrierattr_t *a, int pshared);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_barrierattr_setpshared(
    a: *mut pthread_barrierattr_t,
    pshared: core::ffi::c_int,
) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
设置屏障是否可以跨进程共享。`pshared = 0` 表示 `PTHREAD_PROCESS_PRIVATE`，`pshared = 1` 表示 `PTHREAD_PROCESS_SHARED`。

## 前置条件
- `a` 为非空指针（`!a.is_null()`），指向有效的 `pthread_barrierattr_t` 对象
- `pshared` 必须是 `0` 或 `1`

## 后置条件
- Case 1 `pshared == 0`：`a.__attr = 0`，返回 `0`
- Case 2 `pshared == 1`：`a.__attr = i32::MIN as c_uint`（即 `0x8000_0000`），返回 `0`
- Case 3 `pshared > 1`：返回 `EINVAL`，`a.__attr` 不变

**注意**：此处 `__attr` 设置逻辑不同于 `pthread_condattr_setpshared`。屏障属性直接将 `__attr` 整体设为 `0` 或 `INT_MIN`（而非仅设置最高位）。这是因为屏障属性仅编码进程共享标志，没有其他位需要保留。

## 不变量
无。

## 算法
```rust
pub extern "C" fn pthread_barrierattr_setpshared(
    a: *mut pthread_barrierattr_t,
    pshared: c_int,
) -> c_int {
    if (pshared as c_uint) > 1 {
        return EINVAL;
    }
    unsafe {
        (*a).__attr = if pshared == 0 { 0 } else { i32::MIN as c_uint };
    }
    0
}
```

## Rust 内部设计要点
- 使用 `pshared as c_uint` 进行无符号比较，消除有符号溢出风险
- 屏障属性仅编码进程共享标志，`__attr` 整体覆盖（不保留其他位）
- `i32::MIN as c_uint` 得到 `0x8000_0000`，与 C 的 `INT_MIN` 等效
- 仅在解引用裸指针时使用 `unsafe`

---

/* Rely */
[RELY]
Predefined Types:
  pthread_barrierattr_t            // #[repr(C)] 屏障属性类型
  core::ffi::c_int / c_uint       // Rust 核心库 C FFI 类型

Predefined Constants:
  EINVAL                           // POSIX 错误码（来自 <errno.h>）

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_barrierattr_setpshared(a: *mut pthread_barrierattr_t, pshared: c_int) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
