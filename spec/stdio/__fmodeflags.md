# \_\_fmodeflags.c 规约

> musl libc 内部工具函数。将 `fopen` 风格的 mode 字符串转换为 `open()` 系统调用的标志位。

---

## 依赖图

```
__fmodeflags
  └─> strchr  (<string.h>)
```

---

## 函数规约

### 1. \_\_fmodeflags

```c
int __fmodeflags(const char *mode);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。由 `fopen`、`freopen` 等内部实现调用。

#### Intent

将 C 标准库的 `fopen` mode 字符串（如 `"r"`、`"w+"`、`"a+xe"`）转换为底层 `open()`/`openat()` 系统调用所需的 `O_RDONLY`/`O_WRONLY`/`O_RDWR` 等标志位组合。

#### 前置条件

- `mode`: 非 NULL 的合法 mode 字符串（首字符为 `'r'`、`'w'` 或 `'a'`）
- mode 字符串以 null 结尾

#### 后置条件

- 返回组合了对应 `open` 标志的 `int` 值
- 不修改全局状态
- 不设置 errno

#### 系统算法

```
__fmodeflags(mode):
  /* 1. 确定基本访问模式 */
  if strchr(mode, '+'):
    flags = O_RDWR                 // 读写模式
  else if *mode == 'r':
    flags = O_RDONLY               // 只读模式
  else:
    flags = O_WRONLY               // 只写模式

  /* 2. 附加修饰符 */
  if strchr(mode, 'x'): flags |= O_EXCL
  if strchr(mode, 'e'): flags |= O_CLOEXEC
  if *mode != 'r':     flags |= O_CREAT
  if *mode == 'w':     flags |= O_TRUNC
  if *mode == 'a':     flags |= O_APPEND

  return flags
```

#### Mode 字符含义速查

| 字符 | 标志 | 含义 |
|------|------|------|
| `'r'` | `O_RDONLY` | 只读（文件必须存在） |
| `'w'` | `O_WRONLY \| O_CREAT \| O_TRUNC` | 只写（创建/截断） |
| `'a'` | `O_WRONLY \| O_CREAT \| O_APPEND` | 追加写 |
| `'+'` | 覆盖为 `O_RDWR` | 同时读写 |
| `'x'` | `O_EXCL` | 独占创建（与 `O_CREAT` 一起时，文件已存在则失败） |
| `'e'` | `O_CLOEXEC` | close-on-exec |

#### 依赖

- `strchr()` — 字符串字符查找（`<string.h>`，libc 标准函数）
