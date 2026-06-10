# fgetc.c 规约

> musl libc 标准 IO 单字符读取实现。从 FILE 流中读取一个字符。

---

## 依赖图

```
fgetc (Public)
  └── do_getc (inline, from "getc.h")
        ├── getc_unlocked (宏, 定义于 stdio_impl.h)
        │     └── __uflow (see __uflow.c spec)
        ├── locking_getc (static, from "getc.h")
        │     ├── __lockfile / a_cas / a_swap / __wake
        │     └── getc_unlocked
        └── __pthread_self
```

---

## 函数规约

### fgetc

```c
int fgetc(FILE *f);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

从 FILE 流 `f` 中读取一个 `unsigned char` 类型的字符，以 `int` 返回。若到达文件末尾或发生读取错误，返回 `EOF`。内部通过 `do_getc` 实现智能锁管理。

#### 前置条件

- `f`: 非空 FILE 指针，指向已打开的读模式流
- 流中存在可读数据（或已到达 EOF）

#### 后置条件

- **Case 1 成功读取字符**
  - 返回读取到的字符（以 `unsigned char` 转换的 `int`，范围 0-255）
  - FILE 流位置前进一个字符

- **Case 2 到达文件末尾**
  - 返回 `EOF`（通常为 -1）
  - FILE 流设置 `F_EOF` 标志

- **Case 3 读取错误**
  - 返回 `EOF`
  - FILE 流设置 `F_ERR` 标志
  - `errno` 可能被设置（由底层 `read` 系统调用设置）

#### 系统算法

```
fgetc(f):
  return do_getc(f)
```

参见 [getc.h.md](./getc.h.md) 中 `do_getc` 的完整算法描述。

#### 依赖

- `do_getc(FILE *)` — 智能锁字符读取函数（定义于 `getc.h`）
