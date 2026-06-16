# pthread_attr_setstacksize — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_setstacksize(pthread_attr_t *a, size_t size);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_setstacksize(
    a: *mut pthread_attr_t,
    size: usize,
) -> core::ffi::c_int;
```

---

## 意图

设置线程属性中的栈大小。与 `pthread_attr_setstack` 不同，此函数不指定栈地址——栈将由系统自动分配。调用此函数会将 `_a_stackaddr` 清零，以确保线程创建时使用系统分配的栈，而非之前通过 `pthread_attr_setstack` 设置的自定义地址。

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向已初始化的 `pthread_attr_t`
- `size >= PTHREAD_STACK_MIN`（2048 字节）

## 后置条件

- Case 1 有效大小（`size - PTHREAD_STACK_MIN <= SIZE_MAX / 4`）：
  - `(*a)._a_stackaddr` = `0` — 清除栈地址（表示使用系统分配）
  - `(*a)._a_stacksize` = `size`
  - 返回 `0`
- Case 2 大小超出允许范围（`size - PTHREAD_STACK_MIN > SIZE_MAX / 4`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

## 不变量

- 该函数不访问全局状态
- `_a_stackaddr == 0` 等价于"由系统分配栈"的信号

## 算法

```
pthread_attr_setstacksize(a, size):
  1. if size - PTHREAD_STACK_MIN > SIZE_MAX / 4 { return EINVAL }
  2. (*a)._a_stackaddr = 0   // 标记为系统分配栈
  3. (*a)._a_stacksize = size
  4. return 0
```

**与 `pthread_attr_setstack` 的关键区别**：`setstacksize` 将 `_a_stackaddr` 清零，而 `setstack` 将其设为 `(addr as usize) + size`。零值 `_a_stackaddr` 表示栈地址由系统自动分配，非零值表示使用用户提供的自定义栈地址。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t            // 定义于 pthread_impl 模块
Predefined Macros/Constants:
  PTHREAD_STACK_MIN = 2048  // 定义于 <limits.h>
  SIZE_MAX (= usize::MAX)   // Rust 中对应 usize::MAX
  EINVAL                    // 定义于 <errno.h>

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_attr_setstacksize(a: *mut pthread_attr_t, size: usize) -> c_int;
  // 设置栈大小同时清零栈地址（标记系统分配），返回 0 或 EINVAL
Internal Interface:
  (无内部接口)
