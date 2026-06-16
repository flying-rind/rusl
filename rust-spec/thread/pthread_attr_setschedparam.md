# pthread_attr_setschedparam — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_setschedparam(pthread_attr_t *restrict a, const struct sched_param *restrict param);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_setschedparam(
    a: *mut pthread_attr_t,
    param: *const sched_param,
) -> core::ffi::c_int;
```

---

## 意图

设置线程属性中的调度参数（优先级）。musl 实现仅提取 `sched_priority` 字段，不验证其范围的有效性（有效性检查推迟到线程创建时由内核完成）。

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向已初始化的 `pthread_attr_t`
- `param` 为非空指针（`!param.is_null()`），指向有效的 `struct sched_param`
- `a` 和 `param` 不重叠（`restrict` 约束）

## 后置条件

- Case 1 始终成功：返回 `0`
- `(*a)._a_prio` = `(*param).sched_priority`

## 不变量

- 该函数不访问全局状态
- 不做优先级范围校验，由 `sched_setscheduler` 系统调用在实际创建线程时验证

## 算法

```
pthread_attr_setschedparam(a, param):
  1. (*a)._a_prio = (*param).sched_priority
  2. return 0
```

Rust 实现中通过两个 `unsafe` 块分别解引用 `param` 和 `a`。`sched_param` 需为 `#[repr(C)]` 结构体。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t  // 定义于 pthread_impl 模块
  sched_param     // #[repr(C)] 结构体，定义于 sched 模块，包含 sched_priority: c_int 字段
Predefined Macros/Constants:
  (无)

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_attr_setschedparam(a: *mut pthread_attr_t, param: *const sched_param) -> c_int;
  // 从 sched_param 中复制优先级到属性对象，始终返回 0
Internal Interface:
  (无内部接口)
