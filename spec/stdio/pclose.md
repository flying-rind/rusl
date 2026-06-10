# pclose.c 规约

> musl libc 标准库管道关闭函数实现。关闭 `popen` 打开的流，等待子进程退出并返回其状态。

---

## 依赖图

```
pclose (Public)
  ├── fclose(f)                                    — 关闭 FILE 流 (src/stdio/fclose.c)
  └── __sys_wait4(pid, &status, 0, 0)             — 等待子进程退出 (syscall.h)
```

---

## 函数规约

### 1. pclose

```c
int pclose(FILE *f);
```

[Visibility]: User — `<stdio.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

关闭通过 `popen` 打开的 FILE 流，等待关联的子进程退出，并返回子进程的终止状态。若子进程尚未退出，阻塞直到其退出。

先提取 `f->pipe_pid`（子进程 PID），然后调用 `fclose(f)` 关闭流（flush 缓冲区并关闭管道文件描述符），最后通过 `waitpid` 等待子进程退出并获取状态码。

#### 前置条件

- `f`: 非空 FILE 指针，必须是通过 `popen()` 成功打开的流
- `f->pipe_pid` 存储了有效的子进程 PID
- `f` 尚未被 `pclose` 关闭（重复关闭行为未定义）

#### 后置条件

- **Case 1 子进程正常退出**
  - `f` 被关闭，所有缓冲数据已刷新
  - 子进程被回收（reaped）
  - 返回子进程的退出状态码（由 `waitpid` 报告的 `WEXITSTATUS` 编码后的值）

- **Case 2 `waitpid` 失败**
  - `f` 仍被关闭
  - `errno` 被设置（如 `ECHILD`、`EINTR` 后被真正错误中断）
  - 返回 `-1`

- **Case 3 `waitpid` 被 EINTR 中断**
  - 自动重试 `waitpid`（循环直到不为 `-EINTR`）
  - 最终行为与 Case 1 或 Case 2 相同

#### 系统算法

```
pclose(f):
  1. pid = f->pipe_pid       // 从 FILE 提取子进程 PID
  
  2. fclose(f)               // 关闭 FILE: flush + close fd
  
  3. 循环等待子进程:
     while (r = __sys_wait4(pid, &status, 0, 0)) == -EINTR:
       // 被信号中断，重试
     
  4. if r < 0:               // waitpid 真正失败
        return -1             // (errno 由 __syscall_ret 设置，但此处直接 __sys_wait4)
        
  5. return status           // 返回子进程退出状态
```

注意：`pclose` 直接使用 `__sys_wait4`（而非 `__syscall_ret` 包装的版本），因此返回 `status` 时不会经过 `__syscall_ret` 的 `-1+errno` 转换；返回的 `status` 是原始 `waitpid` 的状态值（可被 `WEXITSTATUS` 等宏解析）。

#### 不变量

- 调用后 `f` 不再有效
- 子进程被回收后不再为僵尸进程

#### 依赖

- `fclose(FILE *f)` — 关闭 FILE 流（定义于 `src/stdio/fclose.c`）
- `__sys_wait4(pid_t pid, int *status, int options, struct rusage *rusage)` — 系统调用层等待进程（定义于 `src/internal/syscall.h`）
- `EINTR` — 错误码：系统调用被信号中断（来自 `<errno.h>`）
- `FILE` 结构体 `pipe_pid` 字段（定义于 `src/internal/stdio_impl.h`）

#### 错误处理

| 场景 | 行为 |
|------|------|
| `waitpid` 被信号中断 (`EINTR`) | 在 while 循环中自动重试，直到非 `EINTR` |
| `waitpid` 真正失败 (< 0) | 返回 `r`（负值），errno 已设置 |
| 子进程正常退出 | 返回 status（可被 `WIFEXITED/WEXITSTATUS` 等宏解析） |
