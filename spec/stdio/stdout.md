# stdout.c 规约

> musl libc 标准输出流的初始化定义。包含 `stdout`（对外导出）、`__stdout_FILE`（内部实现）和 `__stdout_used`（内部哨兵变量）。

---

## 依赖图

```
stdout.c
  ├── stdout (Public) ──> 指向 __stdout_FILE
  ├── __stdout_FILE (Internal) ──> 直接初始化 struct _IO_FILE
  ├── __stdout_used (Internal) ──> 指向 __stdout_FILE
  └── (引用函数指针: __stdout_write, __stdio_seek, __stdio_close)
```

---

## 数据结构定义

### 1. `static unsigned char buf[BUFSIZ+UNGET]`

[Visibility]: Internal — 文件作用域静态变量，不对外导出

标准输出的内部缓冲区。大小为 `BUFSIZ + UNGET`（1024 + 8 = 1032 字节），其中：
- 后 1024 字节（`buf + UNGET` 至结尾）为实际写缓冲区（`buf_size = 1024`）
- 前 8 字节为 `UNGET` 字符回退预留区

### 2. `__stdout_FILE`

```c
hidden FILE __stdout_FILE = {
    .buf = buf+UNGET,
    .buf_size = sizeof buf-UNGET,
    .fd = 1,
    .flags = F_PERM | F_NORD,
    .lbf = '\n',
    .write = __stdout_write,
    .seek = __stdio_seek,
    .close = __stdio_close,
    .lock = -1,
};
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用，标准 C 用户通过 `stdout` 宏间接使用

#### Intent

标准输出的 FILE 结构体定义。

#### 字段说明

| 字段 | 值 | 含义 |
|------|-----|------|
| `.buf` | `buf + UNGET` | 缓冲区起始于预留 8 字节回退空间之后 |
| `.buf_size` | `sizeof buf - UNGET` = 1024 | BUFSIZ 大小的写缓冲区 |
| `.fd` | 1 | 文件描述符 1（标准输出） |
| `.flags` | `F_PERM \| F_NORD` | 永久文件（`F_PERM`）+ 只写（`F_NORD`） |
| `.lbf` | `'\n'` | 行缓冲模式（遇到换行符自动刷新） |
| `.write` | `__stdout_write` | stdout 专用写函数（检测终端行缓冲） |
| `.seek` | `__stdio_seek` | 底层定位操作函数 |
| `.close` | `__stdio_close` | 底层关闭操作函数 |
| `.lock` | -1 | 免锁模式（标准流由 `__stdio_exit` 特殊管理） |

**关键区别**：
- stdout 使用 `__stdout_write`（而非 `__stdio_write`），该函数在写入前检测文件描述符是否为终端，若是终端则强制使用 `'\n'` 行缓冲
- stdout 的行缓冲标志 `lbf = '\n'`，表示遇到换行符时自动刷新

#### 依赖

- `__stdout_write` — stdout 专用写操作（见 `__stdout_write.c` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek.c` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close.c` spec）

---

## 全局符号规约

### 3. stdout

```c
FILE *const stdout = &__stdout_FILE;
```

[Visibility]: User — `<stdio.h>` 声明，用户程序通过 `stdout` 宏直接使用

#### Intent

指向标准输出 FILE 对象的常量指针。用户程序使用此指针向标准输出写入。stdout 默认行缓冲模式。

#### 特点

- `const` 指针：`stdout` 自身不可被修改
- 文件描述符为 1
- 标志为 `F_PERM | F_NORD`：永久、不可读
- 行缓冲（`lbf = '\n'`），缓冲区大小为 `BUFSIZ`（1024 字节）

### 4. \_\_stdout_used

```c
FILE *volatile __stdout_used = &__stdout_FILE;
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用

#### Intent

内部哨兵变量。在程序退出时，`__stdio_exit` 函数通过 `__stdout_used` 获取 stdout 的 FILE 指针来执行最终刷新操作。

如果链接时没有任何stdio相关函数被使用，`__stdout_used` 可能通过弱别名被替换为 NULL。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `BUFSIZ` | 1024 | `<stdio.h>` | 默认缓冲区大小 |
| `UNGET` | 8 | `stdio_impl.h` | 字符回退预留空间 |
| `F_PERM` | 1 | `stdio_impl.h` | 永久流标志 |
| `F_NORD` | 4 | `stdio_impl.h` | 不可读标志（只写） |
