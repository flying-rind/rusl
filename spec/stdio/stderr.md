# stderr.c 规约

> musl libc 标准错误输出流的初始化定义。包含 `stderr`（对外导出）、`__stderr_FILE`（内部实现）和 `__stderr_used`（内部哨兵变量）。

---

## 依赖图

```
stderr.c
  ├── stderr (Public) ──> 指向 __stderr_FILE
  ├── __stderr_FILE (Internal) ──> 直接初始化 struct _IO_FILE
  ├── __stderr_used (Internal) ──> 指向 __stderr_FILE
  └── (引用函数指针: __stdio_write, __stdio_seek, __stdio_close)
```

---

## 数据结构定义

### 1. `static unsigned char buf[UNGET]`

[Visibility]: Internal — 文件作用域静态变量，不对外导出

用于标准错误输出的 8 字节缓冲区。由于 stderr 是无缓冲模式（`buf_size = 0`），该缓冲区仅用于容纳 `UNGET` 预留区（即只用于字符回退操作）。

### 2. `__stderr_FILE`

```c
hidden FILE __stderr_FILE = {
    .buf = buf+UNGET,
    .buf_size = 0,
    .fd = 2,
    .flags = F_PERM | F_NORD,
    .lbf = -1,
    .write = __stdio_write,
    .seek = __stdio_seek,
    .close = __stdio_close,
    .lock = -1,
};
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用，标准 C 用户通过 `stderr` 宏间接使用

#### Intent

标准错误输出的 FILE 结构体定义。

#### 字段说明

| 字段 | 值 | 含义 |
|------|-----|------|
| `.buf` | `buf + UNGET` | 缓冲区起始于预留 8 字节回退空间之后 |
| `.buf_size` | 0 | 无缓冲模式（stderr 默认无缓冲） |
| `.fd` | 2 | 文件描述符 2（标准错误输出） |
| `.flags` | `F_PERM \| F_NORD` | 永久文件（`F_PERM`）+ 只写（`F_NORD`） |
| `.lbf` | -1（EOF）| 非行缓冲（无缓冲） |
| `.write` | `__stdio_write` | 底层写操作函数 |
| `.seek` | `__stdio_seek` | 底层定位操作函数 |
| `.close` | `__stdio_close` | 底层关闭操作函数 |
| `.lock` | -1 | 免锁模式（标准流由 `__stdio_exit` 特殊管理） |

#### 依赖

- `__stdio_write` — 默认 FILE 写操作（见 `__stdio_write.c` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek.c` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close.c` spec）

---

## 全局符号规约

### 3. stderr

```c
FILE *const stderr = &__stderr_FILE;
```

[Visibility]: User — `<stdio.h>` 声明，用户程序通过 `stderr` 宏直接使用

#### Intent

指向标准错误输出 FILE 对象的常量指针。用户程序使用此指针向标准错误输出写入。stderr 默认无缓冲模式。

#### 特点

- `const` 指针：`stderr` 自身不可被修改，但其指向的 FILE 对象是可变的（包含缓冲区位置、标志等运行时状态）
- 文件描述符为 2
- 标志为 `F_PERM | F_NORD`：永久、不可读
- 无缓冲（`buf_size = 0`）

### 4. \_\_stderr_used

```c
FILE *volatile __stderr_used = &__stderr_FILE;
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用

#### Intent

内部哨兵变量。在程序退出时，`__stdio_exit` 函数通过 `__stderr_used`（而非 `stderr`）获取 stderr 的 FILE 指针来执行最终的刷新操作。`volatile` 修饰防止编译器优化对此变量的引用。

如果链接时没有任何stdio相关函数被使用，`__stderr_used` 可能通过弱别名被替换为 NULL（见 `__stdio_exit.c` spec）。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `UNGET` | 8 | `stdio_impl.h` | 字符回退预留空间 |
| `F_PERM` | 1 | `stdio_impl.h` | 永久流标志（不可被 freopen 重新分配） |
| `F_NORD` | 4 | `stdio_impl.h` | 不可读标志 |
