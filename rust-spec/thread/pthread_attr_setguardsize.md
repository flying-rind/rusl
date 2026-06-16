# pthread_attr_setguardsize — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_setguardsize(pthread_attr_t *a, size_t size);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_setguardsize(
    a: *mut pthread_attr_t,
    size: usize,
) -> core::ffi::c_int;
```

---

## 意图

设置线程属性中的守护页大小。守护页位于线程栈末尾，配置为不可访问，用于检测栈溢出。校验规则：`size` 不允许超过 `SIZE_MAX/8`，防止后续在栈大小计算时发生整数溢出。

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向已初始化的 `pthread_attr_t`

## 后置条件

- Case 1 有效值（`size <= SIZE_MAX / 8`）：
  - `(*a)._a_guardsize` = `size`
  - 返回 `0`
- Case 2 超出允许范围（`size > SIZE_MAX / 8`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

## 不变量

- 该函数不访问全局状态
- 上限 `SIZE_MAX/8` 确保在后续栈大小 + 守护页大小计算时不会溢出

## 算法

```
pthread_attr_setguardsize(a, size):
  1. if size > SIZE_MAX / 8 { return EINVAL }
  2. (*a)._a_guardsize = size
  3. return 0
```

Rust 实现中 `SIZE_MAX` 对应 `usize::MAX`。比较 `size > usize::MAX / 8` 在 `usize` 范围内不会溢出（编译期常量表达式）。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t         // 定义于 pthread_impl 模块
Predefined Macros/Constants:
  SIZE_MAX (= usize::MAX)  // Rust 中对应 usize::MAX
  EINVAL                   // 定义于 <errno.h>

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_attr_setguardsize(a: *mut pthread_attr_t, size: usize) -> c_int;
  // 设置守护页大小，size 不可超过 SIZE_MAX/8
Internal Interface:
  (无内部接口)
