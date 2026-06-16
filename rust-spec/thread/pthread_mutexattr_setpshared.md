# pthread_mutexattr_setpshared — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutexattr_setpshared(pthread_mutexattr_t *a, int pshared);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutexattr_setpshared(
    a: *mut pthread_mutexattr_t,
    pshared: core::ffi::c_int
) -> core::ffi::c_int;
```

---

## 类型与常量

| 常量 | 值 | 含义 |
|------|-----|------|
| `PTHREAD_PROCESS_PRIVATE` | 0 | 仅同进程内各线程间共享 |
| `PTHREAD_PROCESS_SHARED` | 1 | 跨进程共享（需放置在共享内存中） |

共享标志存储在 `__attr` 的位 7（`__attr & 128`）。`pshared` 值为 0 或 1，左移 7 位存入。

---

## 意图
设置互斥锁属性对象的进程共享属性。`PTHREAD_PROCESS_SHARED` 表示互斥锁可在不同进程间同步（须位于共享内存段中）；`PTHREAD_PROCESS_PRIVATE` 为默认值。

## 前置条件
- `a` 非空指针（`!a.is_null()`）
- `a` 指向一个已初始化的 `pthread_mutexattr_t`

## 后置条件
- Case 1 `pshared` 为 `PTHREAD_PROCESS_PRIVATE`(0) 或 `PTHREAD_PROCESS_SHARED`(1)：
  - `(*a).__attr` 的位 7 设置为 `pshared << 7`，其他位保持不变
  - 返回值为 `0`
- Case 2 `pshared > 1`（非法值）：
  - `(*a).__attr` 不变
  - 返回值为 `EINVAL`

## 不变量
- 非 process-shared 属性位（type、robust、PI 位）不受此操作影响

## 算法
纯位操作，无系统调用依赖：

```rust
extern "C" fn pthread_mutexattr_setpshared(
    a: *mut pthread_mutexattr_t,
    pshared: core::ffi::c_int
) -> core::ffi::c_int {
    if pshared > 1 {
        return libc_errcode::EINVAL;
    }
    unsafe {
        let attr = &mut (*a).__attr;
        *attr = (*attr & !128) | ((pshared as core::ffi::c_uint) << 7);
    }
    0
}
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pshared {
    Private = 0,
    Shared = 1,
}

pub(crate) fn mutexattr_setpshared(
    a: &mut pthread_mutexattr_t,
    pshared: Pshared
);
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)                         // 纯位操作，无外部依赖

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutexattr_setpshared(a: *mut pthread_mutexattr_t, pshared: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutexattr_setpshared 符号
Internal Interface:
  pub(crate) fn mutexattr_setpshared(a: &mut pthread_mutexattr_t, pshared: Pshared);
                                 // 安全包装，使用枚举替代裸 int，供 crate 内部使用
  pub(crate) enum Pshared;
                                 // 进程共享标志枚举
