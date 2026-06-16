# sem_init.c 规约

> musl libc 匿名信号量初始化函数。

---

## 依赖图

```
sem_init
  (无内部依赖 — 直接操作 sem->__val 数组)
```

---

## 函数规约

### 1. `sem_init`

```c
int sem_init(sem_t *sem, int pshared, unsigned value);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

初始化一个匿名信号量。设置初始值与进程共享标志。

#### 前置条件

- `sem != NULL`，指向待初始化的 `sem_t`
- `value <= SEM_VALUE_MAX`（即 `<= 0x7FFFFFFF`）

#### 后置条件

- Case 1 成功（`value <= SEM_VALUE_MAX`）：
  - `sem->__val[0] = value`（信号量计数值）
  - `sem->__val[1] = 0`（等待者计数）
  - `sem->__val[2] = pshared ? 0 : 128`（私有标志。128 = `FUTEX_PRIVATE` 标志位，`pshared==false` 表示进程内；`pshared==true` 时设为 0，表示跨进程）
  - 返回 `0`
- Case 2 失败（`value > SEM_VALUE_MAX`）：
  - `errno = EINVAL`
  - 返回 `-1`

#### 系统算法

```
sem_init(sem, pshared, value):
  1. 若 value > SEM_VALUE_MAX → 设置 errno = EINVAL，返回 -1
  2. sem->__val[0] = value    （计数值）
  3. sem->__val[1] = 0         （等待者计数）
  4. sem->__val[2] = (pshared ? 0 : 128)   （私有/共享标志）
  5. 返回 0
```

#### 不变量

- `sem->__val[2]` 编码 futex 私有标志：`128`（`FUTEX_PRIVATE`）表示进程内信号量；`0` 表示进程共享信号量

#### 注意

musl 中匿名信号量与有名信号量使用相同的数据结构 `sem_t`，通过 `__val[2]` 区分：匿名信号量为 `128`（进程内 futex），有名信号量为 `0`（mmap 共享内存）。
