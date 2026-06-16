# pthread_attr_setstack — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_setstack(pthread_attr_t *a, void *addr, size_t size);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_setstack(
    a: *mut pthread_attr_t,
    addr: *mut core::ffi::c_void,
    size: usize,
) -> core::ffi::c_int;
```

---

## 意图

同时设置线程属性中的栈地址和栈大小。允许调用者提供一个预分配的内存区域作为线程栈。`addr` 指向栈的最低地址（栈的起始端），存储在属性中的 `_a_stackaddr` 是栈的最高地址 `addr + size`（便于从高地址向低地址增长）。在线程创建时，musl 直接将 `_a_stackaddr` 作为初始栈指针使用。

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向已初始化的 `pthread_attr_t`
- `addr` 为非空指针，指向足够大的内存区域（至少 `size` 字节）
- `size >= PTHREAD_STACK_MIN`（2048 字节）

## 后置条件

- Case 1 有效大小（`size - PTHREAD_STACK_MIN <= SIZE_MAX / 4`）：
  - `(*a)._a_stackaddr` = `(addr as usize) + size` — 存储栈顶地址（高地址端）
  - `(*a)._a_stacksize` = `size`
  - 返回 `0`
- Case 2 大小超出允许范围（`size - PTHREAD_STACK_MIN > SIZE_MAX / 4`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

## 不变量

- 该函数不访问全局状态
- 校验条件 `size - PTHREAD_STACK_MIN > SIZE_MAX/4` 用于防止栈地址计算时溢出，同时将栈大小上限定为约 `SIZE_MAX/4 + 2048`
- `_a_stackaddr` 存储的是 `addr + size`（栈顶）而非 `addr`（栈底），因为栈向低地址增长

## 算法

```
pthread_attr_setstack(a, addr, size):
  1. if size - PTHREAD_STACK_MIN > SIZE_MAX / 4 { return EINVAL }
  2. // 注意：Rust 中 addr as usize 可能需要通过 addr as *const u8 as usize 转换
  3. (*a)._a_stackaddr = (addr as usize) + size
  4. (*a)._a_stacksize = size
  5. return 0
```

**设计说明**：`pthread_attr_getstack` 通过 `_a_stackaddr - size` 反算出栈基址返回给调用者，因此 `_a_stackaddr` 存储的是栈顶（高地址）而非栈底。

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
  extern "C" fn pthread_attr_setstack(a: *mut pthread_attr_t, addr: *mut c_void, size: usize) -> c_int;
  // 设置栈地址（存储为栈顶 addr+size）和栈大小
Internal Interface:
  (无内部接口)
