# pthread_attr_setstack.c 规约

> musl libc 线程属性设置函数（stack）。设置线程的栈地址和栈大小。栈基址存储为"栈顶地址"（高地址端），因为 Linux 栈向低地址增长。

---

## 依赖图

```
pthread_attr_setstack
  (无函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_setstack

```c
int pthread_attr_setstack(pthread_attr_t *a, void *addr, size_t size);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

同时设置线程属性中的栈地址和栈大小。允许调用者提供一个预分配的内存区域作为线程栈。`addr` 指向栈的最低地址（栈的起始端），存储在属性中的 `_a_stackaddr` 是栈的最高地址 `addr + size`（便于从高地址向低地址增长）。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `addr != NULL`，指向足够大的内存区域（至少 `size` 字节）
- `size >= PTHREAD_STACK_MIN`（2048 字节）

#### 后置条件

- Case 1 有效大小（`size - PTHREAD_STACK_MIN <= SIZE_MAX/4`）：
  - `a->_a_stackaddr = (size_t)addr + size` — 存储栈顶地址
  - `a->_a_stacksize = size`
  - 返回 `0`
- Case 2 大小超出允许范围（`size - PTHREAD_STACK_MIN > SIZE_MAX/4`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

#### 不变量

- 该函数不访问全局状态
- 校验条件 `size - PTHREAD_STACK_MIN > SIZE_MAX/4` 用于防止栈地址计算时溢出，同时将栈大小上限定为约 `SIZE_MAX/4 + 2048`

#### 系统算法

```
pthread_attr_setstack(a, addr, size):
  1. if (size - PTHREAD_STACK_MIN > SIZE_MAX/4) return EINVAL
  2. a->_a_stackaddr = (size_t)addr + size   // 存储栈顶（高地址）
  3. a->_a_stacksize = size                   // 存储栈大小
  4. return 0
```

**设计说明**：`_a_stackaddr` 存储的是 `addr + size` 而非 `addr`。这是因为栈向低地址增长，后续 `pthread_attr_getstack` 返回的 `*addr` 通过 `_a_stackaddr - *size` 反算出栈基址。在线程创建时，musl 直接从 `_a_stackaddr` 作为初始栈指针使用。

#### 依赖

- `pthread_impl.h` — 内部头文件，定义 `_a_stackaddr` / `_a_stacksize` 宏
- `PTHREAD_STACK_MIN` — 宏，定义于 `<limits.h>`，值为 `2048`
- `SIZE_MAX` — 宏，定义于 `<stdint.h>` 或 `<limits.h>`
- `EINVAL` — 宏，定义于 `<errno.h>`
