# pthread_getname_np.c 规约

> musl libc 线程名称获取函数。GNU 扩展（`_np` = non-portable）。

---

## 依赖图

```
pthread_getname_np
  ├─> pthread_self()              (see pthread_self.c spec)
  ├─> prctl(PR_GET_NAME, ...)     (Linux 系统调用，来自 <sys/prctl.h>)
  ├─> snprintf()                  (see stdio/snprintf.c spec)
  ├─> pthread_setcancelstate()    (see pthread_cancel.c spec)
  ├─> open()                      (POSIX 系统调用，来自 <fcntl.h>)
  ├─> read()                      (POSIX 系统调用，来自 <unistd.h>)
  ├─> close()                     (POSIX 系统调用，来自 <unistd.h>)
  └─> errno                       (see errno spec)
```

---

## 函数规约

### 1. `pthread_getname_np`

```c
int pthread_getname_np(pthread_t thread, char *name, size_t len);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（GNU 扩展）

#### Intent

获取指定线程的名称。对线程自身使用 `prctl(PR_GET_NAME)` 直接获取，对其他线程通过读取 `/proc/self/task/<tid>/comm` 获取。

#### 前置条件

- `thread` 是有效的 `pthread_t`（未校验）
- `name != NULL`，指向可写缓冲区
- `len >= 16`（线程名最大 16 字符含终止 null）

#### 后置条件

- Case 1 `len < 16`：
  - 返回 `ERANGE`，不修改 `name` 缓冲区
- Case 2 `thread == pthread_self()` 且 `prctl(PR_GET_NAME)` 成功：
  - `name` 缓冲区被填充为当前线程名（null 结尾）
  - 返回 `0`
- Case 3 `thread == pthread_self()` 且 `prctl` 失败：
  - 返回 `errno`
- Case 4 `thread != pthread_self()`：
  - 构造路径 `/proc/self/task/<tid>/comm`
  - 在取消禁用状态下执行文件读取
  - 成功时 `name` 包含线程名（自动去除末尾换行符），返回 `0`
  - 失败时返回对应的 `errno`

#### 系统算法

```
pthread_getname_np(thread, name, len):
  1. 若 len < 16 → 返回 ERANGE
  2. 若 thread == pthread_self()：
     - 调用 prctl(PR_GET_NAME, name, 0, 0, 0)
     - 成功返回 0，失败返回 errno
  3. 否则（其他线程）：
     a. snprintf(f, sizeof f, "/proc/self/task/%d/comm", thread->tid)
     b. 禁用线程取消（pthread_setcancelstate(CANCEL_DISABLE, &cs)）
     c. open(f, O_RDONLY|O_CLOEXEC)
     d. 若 open 成功：read(fd, name, len)，若成功则将末尾换行符替换为 '\0'
     e. 若任意操作失败：status = errno
     f. 若 fd >= 0：close(fd)
     g. 恢复线程取消状态（pthread_setcancelstate(cs, 0)）
     h. 返回 status
```

#### 不变量

- 在 `/proc` 文件操作期间禁用线程取消，确保操作原子性
- 读取成功后总是去除末尾换行符（`/proc/.../comm` 以换行结尾）

#### 依赖

- `pthread_self()` — 获取当前线程标识
- `prctl()` — Linux 进程控制系统调用
- `snprintf()` — 格式化路径字符串
- `pthread_setcancelstate()` — 控制取消状态
- `open()` / `read()` / `close()` — POSIX 文件 I/O
- `errno` — 错误码
