# sem_getvalue.c 规约

> musl libc 信号量当前值查询函数。

---

## 依赖图

```
sem_getvalue
  (无内部依赖 — 直接读取 sem->__val[0])
```

---

## 函数规约

### 1. `sem_getvalue`

```c
int sem_getvalue(sem_t *restrict sem, int *restrict valp);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

获取信号量 `sem` 的当前值。直接读取 `sem->__val[0]` 并使用位掩码 `SEM_VALUE_MAX` 提取有效值部分（低 31 位），忽略符号位（用于等待队列标记）。

#### 前置条件

- `sem != NULL`，指向有效的 `sem_t`
- `valp != NULL`，指向可写的 `int` 变量

#### 后置条件

- `*valp` 被设置为信号量的当前计数值（0 到 `SEM_VALUE_MAX` 之间）
- 返回 `0`

#### 系统算法

```
sem_getvalue(sem, valp):
  1. val = sem->__val[0]
  2. *valp = val & SEM_VALUE_MAX（屏蔽符号位，提取实际值）
  3. 返回 0
```

#### 不变量

- 始终返回 `0`（POSIX 规定此函数不返回错误）
- 返回值为瞬时快照，调用后可能立即改变

#### 注意

`sem->__val[0]` 的位编码为：
- 低 31 位（`& SEM_VALUE_MAX` 即 `& 0x7FFFFFFF`）：信号量计数值
- 第 31 位（bit 31）用作内部等待标记
