# tempnam.c 规约

> musl libc 标准库可定制临时文件名生成函数实现。允许指定目录和前缀生成唯一文件名，并动态分配返回缓冲区。

---

## 依赖图

```
tempnam (Public)
  ├── strlen(dir) / strlen(pfx)               — 计算长度 (<string.h>)
  ├── memcpy(s, dir, dl)                      — 组装路径 (<string.h>)
  ├── __randname(s+l-6)                       — 生成随机文件名后缀 (src/temp/__randname.c)
  ├── __syscall(SYS_readlink, ...)            — 检测文件存在 (syscall.h)
  ├── __syscall(SYS_readlinkat, ...)          — 相对路径检测 (fallback, syscall.h)
  └── strdup(s)                               — 复制最终路径 (<stdlib.h>)
```

---

## 函数规约

### 1. tempnam

```c
char *tempnam(const char *dir, const char *pfx);
```

[Visibility]: User — `<stdio.h>` POSIX 标准函数（XSI 扩展），用户程序可直接调用

#### Intent

生成一个不与现有文件冲突的临时文件的路径名（不创建文件）。与 `tmpnam` 不同，允许调用者指定存放目录和文件名前缀。生成的路径形式为 `<dir>/<pfx>_XXXXXX`（若未指定则使用默认值）。返回的是 `strdup` 分配的动态内存，调用者负责 `free`。

**警告**: 与 `tmpnam` 类似，此函数存在 TOCTOU 竞态条件，被 POSIX 标记为过时。请使用 `mkstemp` 替代。

#### 前置条件

- `dir`: 可为 `NULL` → 默认使用 `P_tmpdir`
- `pfx`: 可为 `NULL` → 默认使用 `"temp"`
- `dir + "/" + pfx + "_" + 6位随机字符` 的长度 < `PATH_MAX`

#### 后置条件

- **Case 1 成功生成**
  - 返回指向 `strdup` 分配的唯一路径名字符串的指针
  - 该路径在设计上不与当前已有文件冲突
  - 调用者有责任调用 `free()` 释放返回的字符串

- **Case 2 路径过长（`l >= PATH_MAX`）**
  - 返回 `NULL`
  - `errno = ENAMETOOLONG`

- **Case 3 所有尝试失败（100 次内无唯一文件名）**
  - 返回 `NULL`
  - `errno` 为最后一次 `readlink` 系统调用的错误码

#### 系统算法

```
tempnam(dir, pfx):
  1. 默认值处理:
     if !dir: dir = P_tmpdir           // 通常为 "/tmp"
     if !pfx: pfx = "temp"
  
  2. 长度校验:
     dl = strlen(dir)
     pl = strlen(pfx)
     l  = dl + 1 + pl + 1 + 6          // dir + '/' + pfx + '_' + XXXXXXXXXX
     if l >= PATH_MAX:
       errno = ENAMETOOLONG
       return NULL
  
  3. 路径组装:
     s[PATH_MAX]
     memcpy(s, dir, dl)                // 写入目录
     s[dl] = '/'                       // 分隔符
     memcpy(s+dl+1, pfx, pl)          // 写入前缀
     s[dl+1+pl] = '_'                  // 分隔符
     s[l] = '\0'                       // 终止符
  
  4. for try in [0, MAXTRIES=100):
       __randname(s+l-6)               // 替换尾部 6 个字符为随机字母数字
       r = readlink(s, &(char[1]){0}, 1) // 文件存在检测
       if r == -ENOENT:                // 路径不存在
         return strdup(s)              // 复制并返回
  
  5. return NULL  // 所有尝试失败
```

#### 依赖

- `strlen(const char *s)` — 计算字符串长度（来自 `<string.h>`）
- `memcpy(void *dst, const void *src, size_t n)` — 内存复制（来自 `<string.h>`）
- `__randname(char *template)` — 生成随机文件名后缀（定义于 `src/temp/__randname.c`）
- `__syscall(SYS_readlink, ...)` / `__syscall(SYS_readlinkat, AT_FDCWD, ...)` — 读取符号链接系统调用，此处用于文件存在检测（定义于 `src/internal/syscall.h`）
- `strdup(const char *s)` — 动态分配字符串副本（来自 `<stdlib.h>`）
- `P_tmpdir` — 系统默认临时目录路径（来自 `<stdio.h>`）
- `PATH_MAX` — 路径名最大长度常量（来自 `<limits.h>`）
- `ENOENT` / `ENAMETOOLONG` — 错误码（来自 `<errno.h>`）
- `AT_FDCWD` — 当前工作目录常量（来自 `<fcntl.h>`，条件编译分支）

#### 安全警告

- **TOCTOU 竞态条件**: 与 `tmpnam` 相同，生成文件名和创建文件之间不是原子操作
- **返回动态内存**: 调用者必须 `free()` 返回值，否则内存泄漏
