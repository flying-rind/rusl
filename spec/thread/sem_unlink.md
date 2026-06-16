# sem_unlink.c 规约

> musl libc 有名信号量解除链接函数。直接委托给 POSIX 共享内存卸载。

---

## 依赖图

```
sem_unlink
  └─> shm_unlink()    (POSIX 系统调用，来自 <sys/mman.h>)
```

---

## 函数规约

### 1. `sem_unlink`

```c
int sem_unlink(const char *name);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

从系统中移除有名信号量。实际委托给 `shm_unlink()`，删除 `/dev/shm` 中对应的共享内存文件。

#### 前置条件

- `name != NULL`，指向 '/' 开头的信号量名称

#### 后置条件

- Case 1 成功：
  - 名称从系统中移除，返回 `0`
  - 已打开该信号量的线程仍可继续使用，但不再能通过 `sem_open` 打开
- Case 2 失败：
  - 返回 `-1`，`errno` 设置对应错误

#### 系统算法

```
sem_unlink(name):
  直接返回 shm_unlink(name)
```

#### 不变量

- 无额外逻辑，纯粹的一对一委托
- 信号量名称被移除后不影响已存在的 mmap 映射

#### 依赖

- `shm_unlink()` — POSIX 共享内存卸载系统调用
