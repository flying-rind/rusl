# pthread_attr_setdetachstate — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_setdetachstate(pthread_attr_t *a, int state);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_setdetachstate(
    a: *mut pthread_attr_t,
    state: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图

设置线程属性中的分离状态。
- `PTHREAD_CREATE_JOINABLE` (0)：线程可被等待（`pthread_join`）
- `PTHREAD_CREATE_DETACHED` (1)：线程退出后自动释放资源

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向已初始化的 `pthread_attr_t`

## 后置条件

- Case 1 有效值（`state <= 1`）：
  - `(*a)._a_detach` = `state`
  - 返回 `0`
- Case 2 无效值（`state > 1`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

## 不变量

- 该函数不访问全局状态
- `state` 的有效取值范围为 {0, 1}

## 算法

```
pthread_attr_setdetachstate(a, state):
  1. if state > 1 { return EINVAL }
  2. (*a)._a_detach = state
  3. return 0
```

Rust 实现中通过 `unsafe` 块解引用 `a` 写入对应字段。值与 C ABI 的 `unsigned` 比较语义一致（`state > 1U` 等效于 `state > 1`，因为 `state` 为 `c_int` 类型不可为负）。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t         // 定义于 pthread_impl 模块
Predefined Macros/Constants:
  PTHREAD_CREATE_JOINABLE = 0
  PTHREAD_CREATE_DETACHED = 1
  EINVAL                  // 定义于 <errno.h>

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_attr_setdetachstate(a: *mut pthread_attr_t, state: c_int) -> c_int;
  // 设置分离状态，仅接受 0 或 1
Internal Interface:
  (无内部接口)
