# pthread_attr_setschedpolicy — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_setschedpolicy(pthread_attr_t *a, int policy);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_setschedpolicy(
    a: *mut pthread_attr_t,
    policy: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图

设置线程属性中的调度策略（如 `SCHED_OTHER`、`SCHED_FIFO`、`SCHED_RR`）。musl 实现直接存储策略值，不验证其有效性（有效性检查推迟到线程创建时由内核完成）。

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向已初始化的 `pthread_attr_t`
- `policy` 应为 POSIX 定义的调度策略常量，但本函数不做验证

## 后置条件

- Case 1 始终成功：返回 `0`
- `(*a)._a_policy` = `policy`

## 不变量

- 该函数不访问全局状态
- 不做策略值范围校验

## 算法

```
pthread_attr_setschedpolicy(a, policy):
  1. (*a)._a_policy = policy
  2. return 0
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t   // 定义于 pthread_impl 模块
Predefined Macros/Constants:
  SCHED_OTHER      // 通常为 0
  SCHED_FIFO       // 通常为 1
  SCHED_RR         // 通常为 2
  (定义于 <sched.h>)

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_attr_setschedpolicy(a: *mut pthread_attr_t, policy: c_int) -> c_int;
  // 设置调度策略，不做验证，始终返回 0
Internal Interface:
  (无内部接口)
