# fputc.c 规约

> musl libc 标准 IO 单字符写入实现。将一个字符写入 FILE 流。

---

## 依赖图

```
fputc (Public)
  └── do_putc (inline, from "putc.h")
        ├── putc_unlocked (宏, 定义于 stdio_impl.h)
        │     └── __overflow (see __overflow.c spec)
        ├── locking_putc (static, from "putc.h")
        │     ├── __lockfile / a_cas / a_swap / __wake
        │     └── putc_unlocked
        └── __pthread_self
```

---

## 函数规约

### fputc

```c
int fputc(int c, FILE *f);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

将字符 `c`（转换为 `unsigned char`）写入 FILE 流 `f`。内部通过 `do_putc` 实现智能锁管理。

#### 前置条件

- `c`: 要写入的字符（以 `int` 传递，高位字节被忽略，仅低 8 位有效）
- `f`: 非空 FILE 指针，指向已打开的写模式流

#### 后置条件

- **Case 1 成功写入**
  - 返回写入的字符（以 `unsigned char` 转换的 `int`，范围 0-255）
  - 字符已写入流缓冲区或通过底层 write 写入

- **Case 2 写入错误**
  - 返回 `EOF`（通常为 -1）
  - FILE 流设置 `F_ERR` 标志
  - `errno` 可能被设置

#### 系统算法

```
fputc(c, f):
  return do_putc(c, f)
```

参见 [putc.h.md](./putc.h.md) 中 `do_putc` 的完整算法描述。

#### 不变量

- 对于行缓冲流，遇到 `\n` 时缓冲区会被刷出

#### 依赖

- `do_putc(int, FILE *)` — 智能锁字符写入函数（定义于 `putc.h`）
