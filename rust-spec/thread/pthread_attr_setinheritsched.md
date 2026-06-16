# pthread_attr_setinheritsched — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_setinheritsched(pthread_attr_t *a, int inherit);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_setinheritsched(
    a: *mut pthread_attr_t,
    inherit: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图

设置线程属性中的调度继承策略。
- `PTHREAD_INHERIT_SCHED` (0)：新线程继承创建线程的调度属性
- `PTHREAD_EXPLICIT_SCHED` (1)：新线程使用属性对象中显式设置的调度属性

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向已初始化的 `pthread_attr_t`

## 后置条件

- Case 1 有效值（`inherit <= 1`）：
  - `(*a)._a_sched` = `inherit`
  - 返回 `0`
- Case 2 无效值（`inherit > 1`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

## 不变量

- 该函数不访问全局状态

## 算法

```
pthread_attr_setinheritsched(a, inherit):
  1. if inherit > 1 { return EINVAL }
  2. (*a)._a_sched = inherit
  3. return 0
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t            // 定义于 pthread_impl 模块
Predefined Macros/Constants:
  PTHREAD_INHERIT_SCHED  = 0
  PTHREAD_EXPLICIT_SCHED = 1
  EINVAL                 // 定义于 <errno.h>

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_attr_setinheritsched(a: *mut pthread_attr_t, inherit: c_int) -> c_int;
  // 设置调度继承策略，仅接受 0 或 1
Internal Interface:
  (无内部接口)
