# pthread_attr_setscope.c 规约

> musl libc 线程属性设置函数（scope）。设置线程的竞争范围。Linux 内核仅支持系统级调度（1:1 线程模型），因此只接受 `PTHREAD_SCOPE_SYSTEM`。

---

## 依赖图

```
pthread_attr_setscope
  (无函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_setscope

```c
int pthread_attr_setscope(pthread_attr_t *a, int scope);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置线程属性中的竞争范围（contention scope）。

- `PTHREAD_SCOPE_SYSTEM` (0)：线程与系统中所有线程竞争 CPU 资源（Linux 1:1 模型）
- `PTHREAD_SCOPE_PROCESS` (1)：线程仅与同一进程内线程竞争 CPU 资源（Linux 不支持）

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`

#### 后置条件

- Case 1 `scope == PTHREAD_SCOPE_SYSTEM`：返回 `0`，属性对象不修改（始终是唯一有效值）
- Case 2 `scope == PTHREAD_SCOPE_PROCESS`：返回 `ENOTSUP`，Linux 不支持进程级竞争范围
- Case 3 其他无效值：返回 `EINVAL`
- 所有 case 中属性对象均不被修改

#### 不变量

- musl/Linux 下仅支持 `PTHREAD_SCOPE_SYSTEM`，该约束跨所有线程操作不变
- 该函数不修改属性对象（scope 值不在 `pthread_attr_t` 中存储，而是在 `pthread_attr_getscope` 中硬编码返回 `PTHREAD_SCOPE_SYSTEM`）

#### 系统算法

```
pthread_attr_setscope(a, scope):
  1. switch (scope):
        case PTHREAD_SCOPE_SYSTEM: return 0
        case PTHREAD_SCOPE_PROCESS: return ENOTSUP
        default: return EINVAL
```

#### 依赖

- `pthread_impl.h` — 内部头文件
- `PTHREAD_SCOPE_SYSTEM` — 宏，定义于 `<pthread.h>`（值为 0）
- `PTHREAD_SCOPE_PROCESS` — 宏，定义于 `<pthread.h>`（值为 1）
- `EINVAL` — 宏，定义于 `<errno.h>`
- `ENOTSUP` — 宏，定义于 `<errno.h>`
