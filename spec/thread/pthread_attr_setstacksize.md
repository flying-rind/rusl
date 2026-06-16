# pthread_attr_setstacksize.c 规约

> musl libc 线程属性设置函数（stacksize）。设置线程栈大小，同时清除之前可能设置的栈地址（因为新栈将由系统分配，不能同时指定自定义地址）。

---

## 依赖图

```
pthread_attr_setstacksize
  (无函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_setstacksize

```c
int pthread_attr_setstacksize(pthread_attr_t *a, size_t size);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置线程属性中的栈大小。与 `pthread_attr_setstack` 不同，此函数不指定栈地址——栈将由系统自动分配。调用此函数会将 `_a_stackaddr` 清零，以确保线程创建时使用系统分配的栈，而非之前通过 `pthread_attr_setstack` 设置的自定义地址。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `size >= PTHREAD_STACK_MIN`（2048 字节）

#### 后置条件

- Case 1 有效大小（`size - PTHREAD_STACK_MIN <= SIZE_MAX/4`）：
  - `a->_a_stackaddr = 0` — 清除栈地址（表示使用系统分配）
  - `a->_a_stacksize = size`
  - 返回 `0`
- Case 2 大小超出允许范围（`size - PTHREAD_STACK_MIN > SIZE_MAX/4`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

#### 不变量

- 该函数不访问全局状态
- `_a_stackaddr == 0` 等价于"由系统分配栈"的信号

#### 系统算法

```
pthread_attr_setstacksize(a, size):
  1. if (size - PTHREAD_STACK_MIN > SIZE_MAX/4) return EINVAL
  2. a->_a_stackaddr = 0   // 标记为系统分配栈
  3. a->_a_stacksize = size
  4. return 0
```

**与 `pthread_attr_setstack` 的关键区别**：`setstacksize` 将 `_a_stackaddr` 清零，而 `setstack` 将其设为 `(size_t)addr + size`。零值 `_a_stackaddr` 表示栈地址由系统自动分配，非零值表示使用用户提供的自定义栈地址。

#### 依赖

- `pthread_impl.h` — 内部头文件，定义 `_a_stackaddr` / `_a_stacksize` 宏
- `PTHREAD_STACK_MIN` — 宏，定义于 `<limits.h>`，值为 `2048`
- `SIZE_MAX` — 宏，定义于 `<stdint.h>` 或 `<limits.h>`
- `EINVAL` — 宏，定义于 `<errno.h>`
