# pthread_attr_setschedparam.c 规约

> musl libc 线程属性设置函数（schedparam）。从 `struct sched_param` 中复制调度优先级到线程属性对象中。

---

## 依赖图

```
pthread_attr_setschedparam
  (无函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_setschedparam

```c
int pthread_attr_setschedparam(pthread_attr_t *restrict a, const struct sched_param *restrict param);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置线程属性中的调度参数（优先级）。musl 实现仅提取 `sched_priority` 字段，不验证其范围的有效性（有效性检查推迟到线程创建时由内核完成）。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `param != NULL`，指向有效的 `struct sched_param`
- `a` 和 `param` 不重叠（`restrict` 约束）

#### 后置条件

- Case 1 始终成功：返回 `0`
- `a->_a_prio = param->sched_priority`

#### 不变量

- 该函数不访问全局状态
- 不做优先级范围校验，由 `sched_setscheduler` 系统调用在实际创建线程时验证

#### 依赖

- `pthread_impl.h` — 内部头文件，定义 `_a_prio` 宏
- `struct sched_param` — 定义于 `<sched.h>`，包含 `int sched_priority` 字段
