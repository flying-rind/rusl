# gethostname.c 规约

> musl libc POSIX 主机名获取函数。`gethostname` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
gethostname (Public)
  └── uname(&uts) — 获取系统信息（含 nodename 字段）
        └── syscall(SYS_uname, &uts) — 内核 uname 系统调用
```

---

## 函数规约

### gethostname

```c
int gethostname(char *name, size_t len);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取当前系统的主机名，写入用户提供的缓冲区 `name`。通过 `uname` 系统调用获取内核记录的 `nodename`（主机名），然后拷贝到用户缓冲区。当主机名超过缓冲区长度时，截断并确保 null 终止。

#### 前置条件

- `name`: 指向至少 `len` 字节可用内存的非空指针
- `len`: 缓冲区大小，`> 0`

#### 后置条件

- **Case 1 成功（主机名长度 < `len`）**
  - `name` 被写入以 null 结尾的完整主机名
  - 返回 0

- **Case 2 成功但被截断（主机名长度 >= `len`）**
  - `name` 被写入截断的主机名，`name[len-1]` 设置为 `'\0'`
  - 如果 `len == 1` 且主机名非空，`name[0]` 被设置为 `'\0'`
  - 返回 0

- **Case 3 `uname` 系统调用失败**
  - 返回 -1
  - `errno` 由 `uname` 设置

#### 系统算法

```
gethostname(name, len):
  struct utsname uts                        // 系统信息结构体
  if uname(&uts): return -1                 // 1. 获取系统信息

  if len > sizeof uts.nodename:            // 2. 限制拷贝长度
    len = sizeof uts.nodename               //    不超过 nodename 字段大小

  for i=0..len-1:                          // 3. 逐字节拷贝
    name[i] = uts.nodename[i]
    if name[i] == '\0': break               //    遇到 null 终止符提前结束

  if i && i == len:                        // 4. 若未找到 null 终止符（截断情况）
    name[i-1] = '\0'                        //    在缓冲区末尾设置 null 终止符

  return 0                                  // 5. 成功
```

即：
1. 声明 `struct utsname uts`，调用 `uname(&uts)` 获取内核系统信息。若失败，返回 -1
2. 计算最大拷贝长度：取 `len` 和 `sizeof(uts.nodename)` 的较小值（`uts.nodename` 通常为 65 字节）
3. 逐字节拷贝 `nodename` 到 `name`：`for (i=0; i<len && (name[i] = uts.nodename[i]); i++);`
   - 这个循环同时进行拷贝和 null 检测
   - 如果 `nodename` 短于 `len`，会在 null 终止处停止
   - 如果 `nodename` 到达 `len` 且该位置不是 null，循环在 `i == len` 时退出（拷贝了所有 `len` 个字节但缺少 null 终止符）
4. 若 `i != 0` 且 `i == len`（说明拷贝了 `len` 个非 null 字节，没有 null 终止符空间），将最后一字节设为 `'\0'`
   - 当 `i == 0` 且 `i == len` 不可能出现（因为 `len > 0` 时循环至少执行一次），但检查 `i` 可防止 `name[-1]` 越界

#### 依赖

- `uname` — 获取系统名称信息（`src/linux/uname.c` 或架构特定实现）
- `struct utsname` — 系统名称结构体，定义在 `<sys/utsname.h>`，其 `nodename` 字段存储主机名
