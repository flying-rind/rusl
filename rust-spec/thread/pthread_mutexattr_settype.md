# pthread_mutexattr_settype — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutexattr_settype(pthread_mutexattr_t *a, int type);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutexattr_settype(
    a: *mut pthread_mutexattr_t,
    type_: core::ffi::c_int
) -> core::ffi::c_int;
```

---

## 类型与常量

| 常量 | 值 | 含义 |
|------|-----|------|
| `PTHREAD_MUTEX_NORMAL` | 0 | 普通锁（无死锁检测，无递归） |
| `PTHREAD_MUTEX_DEFAULT` | 0 | `PTHREAD_MUTEX_NORMAL` 的同义名 |
| `PTHREAD_MUTEX_RECURSIVE` | 1 | 允许同一线程重复加锁 |
| `PTHREAD_MUTEX_ERRORCHECK` | 2 | 检测死锁（重复加锁返回 EDEADLK） |

类型值存储在 `__attr` 的低 2 位（`__attr & 3`）。

---

## 意图
设置互斥锁属性对象的互斥锁类型，决定后续创建的互斥锁在重复加锁时的行为（普通/递归/错误检测）。

## 前置条件
- `a` 非空指针（`!a.is_null()`）
- `a` 指向一个已初始化的 `pthread_mutexattr_t`

## 后置条件
- Case 1 `type` 为 `PTHREAD_MUTEX_NORMAL`(0)、`PTHREAD_MUTEX_RECURSIVE`(1) 或 `PTHREAD_MUTEX_ERRORCHECK`(2)：
  - `(*a).__attr` 的低 2 位被替换为 `type`，其他位保持不变
  - 返回值为 `0`
- Case 2 `type` 为其他非法值：
  - `(*a).__attr` 不变
  - 返回值为 `EINVAL`

## 不变量
- 非 type 属性位（robust 位、PI 位、process-shared 位）不受此操作影响

## 算法
纯位操作，无系统调用依赖：

```rust
extern "C" fn pthread_mutexattr_settype(
    a: *mut pthread_mutexattr_t,
    type_: core::ffi::c_int
) -> core::ffi::c_int {
    if (type_ as core::ffi::c_uint) > 2 {
        return libc_errcode::EINVAL;
    }
    unsafe {
        let attr = &mut (*a).__attr;
        *attr = (*attr & !3) | (type_ as core::ffi::c_uint);
    }
    0
}
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum MutexType {
    Normal = 0,
    Default = 0,
    Recursive = 1,
    ErrorCheck = 2,
}

pub(crate) fn mutexattr_settype(
    a: &mut pthread_mutexattr_t,
    mtype: MutexType
);
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)                         // 纯位操作，无外部依赖

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutexattr_settype(a: *mut pthread_mutexattr_t, type_: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutexattr_settype 符号
Internal Interface:
  pub(crate) fn mutexattr_settype(a: &mut pthread_mutexattr_t, mtype: MutexType);
                                 // 安全包装，使用枚举替代裸 int，供 crate 内部使用
  pub(crate) enum MutexType;
                                 // 互斥锁类型枚举
