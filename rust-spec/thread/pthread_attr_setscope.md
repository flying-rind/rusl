# pthread_attr_setscope — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_setscope(pthread_attr_t *a, int scope);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_setscope(
    a: *mut pthread_attr_t,
    scope: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图

设置线程属性中的竞争范围（contention scope）。

- `PTHREAD_SCOPE_SYSTEM` (0)：线程与系统中所有线程竞争 CPU 资源（Linux 1:1 模型，唯一支持的模式）
- `PTHREAD_SCOPE_PROCESS` (1)：线程仅与同一进程内线程竞争 CPU 资源（Linux 不支持）

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向已初始化的 `pthread_attr_t`

## 后置条件

- Case 1 `scope == PTHREAD_SCOPE_SYSTEM`：返回 `0`，属性对象不修改
- Case 2 `scope == PTHREAD_SCOPE_PROCESS`：返回 `ENOTSUP`，Linux 不支持进程级竞争范围
- Case 3 其他无效值：返回 `EINVAL`
- 所有 case 中属性对象均不被修改

## 不变量

- musl/Linux 下仅支持 `PTHREAD_SCOPE_SYSTEM`，该约束跨所有线程操作不变
- 该函数不修改属性对象（scope 值不在 `pthread_attr_t` 中存储，而是在 `pthread_attr_getscope` 中硬编码返回 `PTHREAD_SCOPE_SYSTEM`）

## 算法

```
pthread_attr_setscope(a, scope):
  1. match scope:
       PTHREAD_SCOPE_SYSTEM  => return 0
       PTHREAD_SCOPE_PROCESS => return ENOTSUP
       _                     => return EINVAL
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t          // 定义于 pthread_impl 模块
Predefined Macros/Constants:
  PTHREAD_SCOPE_SYSTEM  = 0  // 定义于 <pthread.h>
  PTHREAD_SCOPE_PROCESS = 1  // 定义于 <pthread.h>
  EINVAL                    // 定义于 <errno.h>
  ENOTSUP                   // 定义于 <errno.h>

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_attr_setscope(a: *mut pthread_attr_t, scope: c_int) -> c_int;
  // 仅接受 PTHREAD_SCOPE_SYSTEM，不修改属性对象
Internal Interface:
  (无内部接口)
