# pthread_attr_setinheritsched.c 规约

> musl libc 线程属性设置函数（inheritsched）。设置线程创建时是否继承父线程的调度属性，仅接受 `PTHREAD_INHERIT_SCHED` (0) 或 `PTHREAD_EXPLICIT_SCHED` (1)。

---

## 依赖图

```
pthread_attr_setinheritsched
  (无函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_setinheritsched

```c
int pthread_attr_setinheritsched(pthread_attr_t *a, int inherit);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置线程属性中的调度继承策略。
- `PTHREAD_INHERIT_SCHED` (0)：新线程继承创建线程的调度属性
- `PTHREAD_EXPLICIT_SCHED` (1)：新线程使用属性对象中显式设置的调度属性

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`

#### 后置条件

- Case 1 有效值（`inherit <= 1U`）：
  - `a->_a_sched = inherit`
  - 返回 `0`
- Case 2 无效值（`inherit > 1U`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

#### 不变量

- 该函数不访问全局状态
- 该文件包含 `syscall.h` 但实际未使用系统调用（可能为历史遗留或预留给未来使用）

#### 系统算法

```
pthread_attr_setinheritsched(a, inherit):
  1. if (inherit > 1U) return EINVAL
  2. a->_a_sched = inherit
  3. return 0
```

#### 依赖

- `pthread_impl.h` — 内部头文件，定义 `_a_sched` 宏
- `syscall.h` — 内部头文件（该文件中实际未使用）
- `EINVAL` — 宏，定义于 `<errno.h>`
