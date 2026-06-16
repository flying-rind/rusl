# pthread_setname_np.c 规约

> musl libc 线程名称设置函数。GNU 扩展（`_np` = non-portable）。

---

## 依赖图

```
pthread_setname_np
  ├─> strnlen()                   (see string/strnlen.c spec)
  ├─> pthread_self()              (see pthread_self.c spec)
  ├─> prctl(PR_SET_NAME, ...)     (Linux 系统调用，来自 <sys/prctl.h>)
  ├─> snprintf()                  (see stdio/snprintf.c spec)
  ├─> pthread_setcancelstate()    (see pthread_cancel.c spec)
  ├─> open()                      (POSIX 系统调用，来自 <fcntl.h>)
  ├─> write()                     (POSIX 系统调用，来自 <unistd.h>)
  ├─> close()                     (POSIX 系统调用，来自 <unistd.h>)
  └─> errno                       (see errno spec)
```

---

## 函数规约

### 1. `pthread_setname_np`

```c
int pthread_setname_np(pthread_t thread, const char *name);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（GNU 扩展）

#### Intent

设置指定线程的名称。对线程自身使用 `prctl(PR_SET_NAME)` 直接设置，对其他线程通过写入 `/proc/self/task/<tid>/comm` 设置。

#### 前置条件

- `thread` 是有效的 `pthread_t`（未校验）
- `name != NULL`，指向以 null 结尾、长度不超过 15 字符的字符串

#### 后置条件

- Case 1 `strnlen(name, 16) > 15`：
  - 返回 `ERANGE`，名称不变
- Case 2 `thread == pthread_self()` 且 `prctl(PR_SET_NAME)` 成功：
  - 线程名称已更新，返回 `0`
- Case 3 `thread == pthread_self()` 且 `prctl` 失败：
  - 返回 `errno`
- Case 4 `thread != pthread_self()`：
  - 构造路径 `/proc/self/task/<tid>/comm`
  - 在取消禁用状态下执行文件写入
  - 成功时返回 `0`，失败时返回对应的 `errno`

#### 系统算法

```
pthread_setname_np(thread, name):
  1. len = strnlen(name, 16)
  2. 若 len > 15 → 返回 ERANGE
  3. 若 thread == pthread_self()：
     - 调用 prctl(PR_SET_NAME, name, 0, 0, 0)
     - 成功返回 0，失败返回 errno
  4. 否则（其他线程）：
     a. snprintf(f, sizeof f, "/proc/self/task/%d/comm", thread->tid)
     b. 禁用线程取消（pthread_setcancelstate(CANCEL_DISABLE, &cs)）
     c. open(f, O_WRONLY|O_CLOEXEC)
     d. 若 open 成功：write(fd, name, len)
     e. 若任意操作失败：status = errno
     f. 若 fd >= 0：close(fd)
     g. 恢复线程取消状态（pthread_setcancelstate(cs, 0)）
     h. 返回 status
```

#### 不变量

- 线程名最大长度为 15 字符（Linux 内核限制，16 字节含终止 null）
- 在 `/proc` 文件操作期间禁用线程取消，确保操作原子性

#### 依赖

- `strnlen()` — 计算字符串长度（上限检查）
- `pthread_self()` — 获取当前线程标识
- `prctl()` — Linux 进程控制系统调用
- `snprintf()` — 格式化路径字符串
- `pthread_setcancelstate()` — 控制取消状态
- `open()` / `write()` / `close()` — POSIX 文件 I/O
- `errno` — 错误码
