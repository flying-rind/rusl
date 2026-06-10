# fileno.c 规约

> musl libc 获取文件流底层文件描述符实现。提供 `fileno` 和 POSIX 免锁扩展 `fileno_unlocked`。

---

## 依赖图

```
fileno
  ├─> FLOCK                 (see stdio_impl.h / __lockfile.c spec)
  │     └─> __lockfile      (see __lockfile.c spec)
  └─> FUNLOCK               (see stdio_impl.h / __lockfile.c spec)
        └─> __unlockfile    (see __lockfile.c spec)

fileno_unlocked = weak_alias(fileno)
```

---

## 数据结构分析

`FILE` 结构体中 `fd` 字段由 `__fdopen` 或 `fopen` 在流初始化时设置。有效的 `fd` 值 >= `0`；`< 0` 表示流未与有效文件描述符关联（如某些内存流场景）。

---

## 函数规约

### 1. fileno

```c
int fileno(FILE *f);
```

[Visibility]: User — POSIX 标准函数，声明于 `<stdio.h>`（需 `_POSIX_C_SOURCE >= 200112L`）。用户程序可直接调用。

#### Intent

获取与文件流 `f` 关联的底层文件描述符。可用于在需要文件描述符的系统调用（如 `fcntl`、`fstat`、`ioctl` 等）中直接操作底层文件。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`

#### 后置条件

**Case 1: 成功 — 流有关联的有效文件描述符**
- `f->fd >= 0`
- 返回 `f->fd`（非负整数）
- `errno` 不变

**Case 2: 失败 — 流未关联有效文件描述符**
- `f->fd < 0`
- `errno` 设置为 `EBADF`（错误的文件描述符）
- 返回 `-1`

#### 系统算法

```
fileno(f):
  FLOCK(f)
  fd = f->fd                     // 读取内部文件描述符
  FUNLOCK(f)
  if fd < 0:
    errno = EBADF
    return -1
  return fd
```

#### 不变量

- 仅读取 `f->fd`，不修改任何状态
- 操作在锁保护下原子执行

#### 依赖

- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（`stdio_impl.h`）
- `<errno.h>` — `EBADF`

---

### 2. fileno_unlocked (weak_alias)

```c
weak_alias(fileno, fileno_unlocked);
```

[Visibility]: User — POSIX 扩展函数，声明于 `<stdio.h>`。

- **Intention**: 与 `fileno` 共享同一实现。前置/后置条件完全等同于 `fileno`。
