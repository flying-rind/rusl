# \_\_stdio_seek.c 规约

> musl libc 内部 FILE 默认定位操作实现。作为 `f->seek` 函数指针的默认值，将定位请求直接转发给 `__lseek` 系统调用。

---

## 依赖图

```
__stdio_seek
  └─> __lseek  (<unistd.h>)
```

---

## 函数规约

### 1. \_\_stdio_seek

```c
off_t __stdio_seek(FILE *f, off_t off, int whence);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。作为 `f->seek` 函数指针的默认值，被 `fseek`、`fseeko` 等间接调用。

#### Intent

对文件描述符执行定位操作。这是一个薄封装，直接将 `seek` 操作转发给底层的 `__lseek` 系统调用。

#### 前置条件

- `f`: `FILE*`，其 `fd` 为有效的文件描述符（支持定位的文件）
- `off`: 偏移量
- `whence`: 定位基准，合法值：
  - `SEEK_SET` (0)：文件起始
  - `SEEK_CUR` (1)：当前位置
  - `SEEK_END` (2)：文件末尾

#### 后置条件

**Case 1: 成功**

- 文件偏移量被更新
- 返回新的文件偏移量（从文件起始的字节偏移）

**Case 2: 失败**

- 返回 `-1`（`(off_t)-1`）
- 设置 errno（由 `__lseek` 设置）

#### 系统算法

```
__stdio_seek(f, off, whence):
  return __lseek(f->fd, off, whence)
```

#### 不变量

- 仅转发调用，不修改 `f` 的字段

#### 依赖

- `__lseek()` — 文件定位系统调用封装（`<unistd.h>`，libc 内部函数）
