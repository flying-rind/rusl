# getcwd.c 规约

> musl libc POSIX 获取当前工作目录函数。`getcwd` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getcwd (Public)
  ├── buf==NULL: 使用栈上 PATH_MAX 大小的临时缓冲区
  │     └── syscall(SYS_getcwd, tmp, PATH_MAX)
  │           └── 成功: strdup(tmp) — 堆复制后返回
  └── buf!=NULL:
        ├── size==0: errno=EINVAL, return NULL
        └── syscall(SYS_getcwd, buf, size)
              ├── 成功且结果以 '/' 开头: return buf（或 strdup(tmp)）
              └── 结果为空或不以 '/' 开头: errno=ENOENT, return NULL
```

---

## 函数规约

### getcwd

```c
char *getcwd(char *buf, size_t size);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取调用进程的当前工作目录绝对路径名。

- 若 `buf` 非 NULL：将绝对路径名复制到 `buf`（长度最多 `size` 字节）
- 若 `buf` 为 NULL：动态分配缓冲区（大小至少 `PATH_MAX`），调用者负责 `free()`

#### 前置条件

- `buf`: NULL（要求动态分配）或指向至少 `size` 字节可用内存的指针
- `size`: 若 `buf` 非 NULL 则必须 > 0 且 >= 路径名长度（含 '\0'）

#### 后置条件

- **Case 1 成功**
  - 返回指向绝对路径名（以 '/' 开头）的指针
  - 若 `buf` 为 NULL：返回的指针指向动态分配的内存（调用者必须 `free()`）
  - 若 `buf` 非 NULL：返回值 == `buf`

- **Case 2 `buf` 非 NULL 但 `size` == 0**
  - 返回 NULL
  - `errno` 设置为 `EINVAL`

- **Case 3 系统调用失败或返回结果不以 '/' 开头**
  - 返回 NULL
  - `errno` 设置为 `ENOENT` 或 syscall 返回的错误码

#### 系统算法

```
getcwd(buf, size):
  if !buf:
    buf = tmp[PATH_MAX]                                   // 栈上临时缓冲区
    size = PATH_MAX
  else if !size:
    errno = EINVAL; return NULL                           // 无效参数

  ret = syscall(SYS_getcwd, buf, size)                    // 内核 getcwd 系统调用
  if ret < 0: return NULL                                 // 系统调用失败

  if ret == 0 || buf[0] != '/':                           // 结果无效
    errno = ENOENT; return NULL

  return (buf == tmp) ? strdup(buf) : buf                 // NULL buf → 堆复制
```

#### 不变量

- 成功返回的路径始终以 '/' 开头（绝对路径保证）

#### 依赖

- `SYS_getcwd` — Linux 内核系统调用
- `strdup` — `<stdlib.h>` 字符串堆复制
- `PATH_MAX` — `<limits.h>` 最大路径长度
