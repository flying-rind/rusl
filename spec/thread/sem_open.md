# sem_open.c 规约

> musl libc 有名信号量打开/创建与关闭模块。基于 `/dev/shm` 共享内存文件实现。

---

## 依赖图

```
sem_open
  ├─> __shm_mapname()            (see mman/shm_open.c spec)
  ├─> LOCK() / UNLOCK()          (see lock.h, 保护 semtab)
  ├─> calloc()                   (see malloc/calloc.c spec — malloc wrapper)
  ├─> access()                   (POSIX 系统调用，来自 <unistd.h>)
  ├─> open()                     (POSIX 系统调用，来自 <fcntl.h>)
  ├─> close()                    (POSIX 系统调用，来自 <unistd.h>)
  ├─> fstat()                    (POSIX 系统调用，来自 <sys/stat.h>)
  ├─> mmap()                     (POSIX 系统调用，来自 <sys/mman.h>)
  ├─> munmap()                   (POSIX 系统调用，来自 <sys/mman.h>)
  ├─> write()                    (POSIX 系统调用，来自 <unistd.h>)
  ├─> link()                     (POSIX 系统调用，来自 <unistd.h>)
  ├─> unlink()                   (POSIX 系统调用，来自 <unistd.h>)
  ├─> clock_gettime()            (POSIX 系统调用，来自 <time.h>)
  ├─> snprintf()                 (see stdio/snprintf.c spec)
  ├─> sem_init()                 (see sem_init.c spec)
  ├─> pthread_setcancelstate()   (see pthread_cancel.c spec)
  └─> errno                      (see errno spec)

sem_close
  ├─> LOCK() / UNLOCK()
  └─> munmap()
```

---

## 全局状态

### `semtab` (static)

```c
static struct {
    ino_t ino;
    sem_t *sem;
    int refcnt;
} *semtab;
```

[Visibility]: Internal (不导出) — 文件作用域 static 变量

#### Intent

有名信号量映射表。记录所有通过 `sem_open` 打开的命名信号量。每个条目跟踪：
- `ino`：底层共享内存文件的 inode 号（用于去重）
- `sem`：mmap 映射的信号量指针
- `refcnt`：引用计数（多个 `sem_open` 同一名称会共享映射）

### `lock` (static)

```c
static volatile int lock[1];
volatile int *const __sem_open_lockptr = lock;
```

[Visibility]: Internal (不导出) — `__sem_open_lockptr` 被导出用于 fork 处理

#### Intent

保护 `semtab` 表访问的自旋锁（通过 `LOCK(lock)` / `UNLOCK(lock)` 使用）。

---

## 函数规约

### 1. `sem_open`

```c
sem_t *sem_open(const char *name, int flags, ...);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

打开或创建有名信号量。若 `flags` 包含 `O_CREAT`，则可变参数中需提供 `mode_t mode` 和 `unsigned value`。

以 `/dev/shm/<prefixed_name>` 文件支持命名信号量，通过 `mmap(MAP_SHARED)` 实现跨进程共享。

#### 前置条件

- `name != NULL`，以 '/' 开头（如 `/mysem`）
- 若 `flags & O_CREAT`：
  - 可变参数包含 `mode_t mode` 和 `unsigned value`
  - `value <= SEM_VALUE_MAX`

#### 后置条件

- Case 1 成功：
  - 返回指向 `sem_t` 的指针（mmap 映射的共享内存区域）
  - 信号量已初始化并可跨进程使用
  - `semtab` 表中增加引用
- Case 2 失败：
  - 返回 `SEM_FAILED`（即 `(sem_t *)0`）
  - `errno` 设置为相应错误码

#### 系统算法

```
sem_open(name, flags, ...):
  1. __shm_mapname(name, buf) 将名称转换为 /dev/shm/ 路径
     失败 → 返回 SEM_FAILED
  2. LOCK(lock) 获取 semtab 锁
  3. 若 semtab 未分配 → calloc 分配 SEM_NSEMS_MAX 条目
     分配失败 → UNLOCK, 返回 SEM_FAILED
  4. 在 semtab 中寻找空闲槽位（refcnt 为 0 且 sem 为 NULL）
     若无空闲槽 → errno = EMFILE, UNLOCK, 返回 SEM_FAILED
  5. 预留槽位：semtab[slot].sem = (sem_t *)-1（哨兵值）
  6. UNLOCK(lock)
  7. 若 flags == (O_CREAT|O_EXCL) 且 access(name, F_OK) == 0：
     errno = EEXIST → 跳转 fail
  8. 循环尝试：
     a. 若非 (O_CREAT|O_EXCL)：
        尝试 open(name, O_RDWR|...)
        若成功 → fstat + mmap(MAP_SHARED) → 跳出循环
        若非 ENOENT → 跳转 fail
     b. 若 flags 不含 O_CREAT → 跳转 fail
     c. 首次进入：va_start 解析 mode 和 value
       若 value > SEM_VALUE_MAX → errno = EINVAL → 跳转 fail
     d. sem_init(&newsem, 1, value) 初始化临时信号量
     e. 用 clock_gettime 生成唯一临时文件名 /dev/shm/tmp-<nsec>
     f. open(tmp, O_CREAT|O_EXCL|...)
        若 EEXIST → 重试（continue）
        其他错误 → 跳转 fail
     g. write(fd, &newsem, sizeof newsem)
     h. fstat + mmap(MAP_SHARED)
     i. close(fd) → link(tmp, name) → unlink(tmp)
        若 link 返回 EEXIST 且非 O_EXCL 模式 → munmap → 重试
        其他错误 → 跳转 fail
        link 成功 → 跳出循环
  9. LOCK(lock)
     在 semtab 中查找是否已有相同 inode 的映射
     若有 → munmap 当前映射，转用已有映射
     若无 → 将当前映射登记到预留槽位
     slot->refcnt++, slot->sem = map, slot->ino = st.st_ino
  10. UNLOCK(lock) → 恢复取消状态 → 返回 map

fail:
  恢复取消状态 → LOCK(lock) → semtab[slot].sem = 0 → UNLOCK(lock) → 返回 SEM_FAILED
```

#### 不变量

- 所有共享同一名称的 `sem_open` 返回相同的 mmap 映射（通过 inode 去重）
- 操作期间禁用线程取消（`PTHREAD_CANCEL_DISABLE`）
- 通过临时文件的 `link` + `unlink` 保证原子创建语义
- `semtab` 最大 `SEM_NSEMS_MAX`（256）个条目，`refcnt` 之和不超过 `INT_MAX`

#### 依赖

- `__shm_mapname()` — 将 posix 名称映射为 `/dev/shm` 文件路径
- `LOCK()` / `UNLOCK()` — 自旋锁（保护 semtab）
- `calloc()` — 分配 semtab 表（通过 malloc 包装）
- `sem_init()` — 初始化信号量结构
- `pthread_setcancelstate()` — 取消状态控制
- `access()` / `open()` / `close()` / `write()` / `link()` / `unlink()` — 文件系统操作
- `fstat()` — 获取文件 inode 信息
- `mmap()` / `munmap()` — 共享内存映射
- `clock_gettime()` — 获取纳秒级时间戳（生成唯一临时文件名）
- `snprintf()` — 格式化路径字符串
- `errno` — 错误码

---

### 2. `sem_close`

```c
int sem_close(sem_t *sem);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

关闭有名信号量。减少引用计数，当引用计数降为 0 时解除 mmap 映射并释放 semtab 条目。

#### 前置条件

- `sem` 是之前通过 `sem_open` 返回的有效指针

#### 后置条件

- Case 1 引用计数 > 1 递减后：
  - `refcnt--`，不解除映射，返回 `0`
- Case 2 引用计数降为 0：
  - `semtab[i].sem = 0`，`semtab[i].ino = 0`
  - `munmap(sem, sizeof *sem)` 解除映射
  - 返回 `0`

#### 系统算法

```
sem_close(sem):
  1. LOCK(lock)
  2. 在 semtab 中查找 semtab[i].sem == sem
  3. --semtab[i].refcnt
  4. 若 refcnt != 0 → UNLOCK, 返回 0
  5. semtab[i].sem = 0; semtab[i].ino = 0
  6. UNLOCK(lock)
  7. munmap(sem, sizeof *sem)
  8. 返回 0
```

#### 依赖

- `LOCK()` / `UNLOCK()` — 保护 semtab
- `munmap()` — 解除共享内存映射
