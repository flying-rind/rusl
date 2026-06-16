# pthread_attr_setschedpolicy.c 规约

> musl libc 线程属性设置函数（schedpolicy）。设置线程的调度策略（如 SCHED_FIFO、SCHED_RR、SCHED_OTHER）。

---

## 依赖图

```
pthread_attr_setschedpolicy
  (无函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_setschedpolicy

```c
int pthread_attr_setschedpolicy(pthread_attr_t *a, int policy);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置线程属性中的调度策略。musl 实现直接存储策略值，不验证其有效性（有效性检查推迟到线程创建时由内核完成）。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `policy` 应为 POSIX 定义的调度策略常量（如 `SCHED_OTHER`, `SCHED_FIFO`, `SCHED_RR`），但本函数不做验证

#### 后置条件

- Case 1 始终成功：返回 `0`
- `a->_a_policy = policy`

#### 不变量

- 该函数不访问全局状态
- 不做策略值范围校验

#### 依赖

- `pthread_impl.h` — 内部头文件，定义 `_a_policy` 宏
