# pthread_condattr_init -- Rust 接口归约

## 原始 C 接口
```c
int pthread_condattr_init(pthread_condattr_t *a);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_condattr_init(a: *mut pthread_condattr_t) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
将条件变量属性对象初始化为默认值。默认值包括：时钟为 `CLOCK_REALTIME`（`0`），非进程共享。

## 前置条件
- `a` 为非空指针（`!a.is_null()`），指向未初始化的 `pthread_condattr_t` 对象

## 后置条件
- 始终返回 `0`（成功）
- `*a` 被零初始化：`a.__attr = 0`
- 零值对应的默认语义：
  - 时钟类型 = `CLOCK_REALTIME`（`__attr` 低 31 位为 0）
  - 非进程共享（`__attr` 最高位为 0）

## 不变量
无。

## 算法
```rust
pub extern "C" fn pthread_condattr_init(a: *mut pthread_condattr_t) -> c_int {
    // 零初始化属性对象
    unsafe { (*a).__attr = 0; }
    0
}
```

## Rust 内部设计要点
- `pthread_condattr_t` 定义为 `#[repr(C)]` 结构体，内存布局与 C 一致
- 零初始化对应默认语义：时钟为 `CLOCK_REALTIME`、非进程共享
- 仅在写入 `__attr` 字段时使用 `unsafe` 解引用裸指针

---

/* Rely */
[RELY]
Predefined Types:
  pthread_condattr_t               // #[repr(C)] 条件变量属性类型
  core::ffi::c_int                // Rust 核心库 C FFI 类型

Predefined Constants:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_condattr_init(a: *mut pthread_condattr_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
