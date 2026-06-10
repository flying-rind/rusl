# stdin.c 规约

> musl libc 标准输入流的初始化定义。包含 `stdin`（对外导出）、`__stdin_FILE`（内部实现）和 `__stdin_used`（内部哨兵变量）。

---

## 依赖图

```
stdin.c
  ├── stdin (Public) ──> 指向 __stdin_FILE
  ├── __stdin_FILE (Internal) ──> 直接初始化 struct _IO_FILE
  ├── __stdin_used (Internal) ──> 指向 __stdin_FILE
  └── (引用函数指针: __stdio_read, __stdio_seek, __stdio_close)
```

---

## 数据结构定义

### 1. `static unsigned char buf[BUFSIZ+UNGET]`

[Visibility]: Internal — 文件作用域静态变量，不对外导出

标准输入的内部缓冲区。大小为 `BUFSIZ + UNGET`（1024 + 8 = 1032 字节），其中：
- 后 1024 字节（`buf + UNGET` 至结尾）为实际读缓冲区（`buf_size = 1024`）
- 前 8 字节为 `UNGET` 字符回退预留区

### 2. `__stdin_FILE`

```c
hidden FILE __stdin_FILE = {
    .buf = buf+UNGET,
    .buf_size = sizeof buf-UNGET,
    .fd = 0,
    .flags = F_PERM | F_NOWR,
    .read = __stdio_read,
    .seek = __stdio_seek,
    .close = __stdio_close,
    .lock = -1,
};
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用，标准 C 用户通过 `stdin` 宏间接使用

#### Intent

标准输入的 FILE 结构体定义。

#### 字段说明

| 字段 | 值 | 含义 |
|------|-----|------|
| `.buf` | `buf + UNGET` | 缓冲区起始于预留 8 字节回退空间之后 |
| `.buf_size` | `sizeof buf - UNGET` = 1024 | BUFSIZ 大小的读缓冲区 |
| `.fd` | 0 | 文件描述符 0（标准输入） |
| `.flags` | `F_PERM \| F_NOWR` | 永久文件（`F_PERM`）+ 不可写（`F_NOWR`） |
| `.read` | `__stdio_read` | 底层读操作函数 |
| `.seek` | `__stdio_seek` | 底层定位操作函数 |
| `.close` | `__stdio_close` | 底层关闭操作函数 |
| `.lock` | -1 | 免锁模式（标准流由 `__stdio_exit` 特殊管理） |

#### 依赖

- `__stdio_read` — 默认 FILE 读操作（见 `__stdio_read.c` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek.c` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close.c` spec）

---

## 全局符号规约

### 3. stdin

```c
FILE *const stdin = &__stdin_FILE;
```

[Visibility]: User — `<stdio.h>` 声明，用户程序通过 `stdin` 宏直接使用

#### Intent

指向标准输入 FILE 对象的常量指针。用户程序使用此指针从标准输入读取。stdin 默认全缓冲模式（`buf_size = BUFSIZ`），`lbf` 字段默认 0（非行缓冲，即全缓冲）。

#### 特点

- `const` 指针：`stdin` 自身不可被修改，但其指向的 FILE 对象是可变的
- 文件描述符为 0
- 标志为 `F_PERM | F_NOWR`：永久、不可写
- 缓冲区大小为 `BUFSIZ`（1024 字节）

### 4. \_\_stdin_used

```c
FILE *volatile __stdin_used = &__stdin_FILE;
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用

#### Intent

内部哨兵变量。在程序退出时，`__stdio_exit` 函数通过 `__stdin_used` 获取 stdin 的 FILE 指针来执行最终刷新操作。

如果链接时没有任何stdio相关函数被使用，`__stdin_used` 可能通过弱别名被替换为 NULL。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `BUFSIZ` | 1024 | `<stdio.h>` | 默认缓冲区大小 |
| `UNGET` | 8 | `stdio_impl.h` | 字符回退预留空间 |
| `F_PERM` | 1 | `stdio_impl.h` | 永久流标志 |
| `F_NOWR` | 8 | `stdio_impl.h` | 不可写标志 |
