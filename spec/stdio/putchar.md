# putchar.c 规约

> musl libc 标准输出单字符写入实现。将一个字符写入 `stdout`。

---

## 依赖图

```
putchar (Public)
  └── do_putc (inline, from "putc.h")
        ├── putc_unlocked (宏)
        ├── locking_putc (static)
        └── __pthread_self

stdout (全局变量, 来自 <stdio.h>)
```

---

## 函数规约

### putchar

```c
int putchar(int c);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

将字符 `c` 写入标准输出流 `stdout`。等价于 `putc(c, stdout)` 或 `fputc(c, stdout)`。

#### 前置条件

- `c`: 要写入的字符（以 `int` 传递，内部转为 `unsigned char`）
- `stdout` 已正确初始化并处于可写状态

#### 后置条件

- **Case 1 成功写入**
  - 返回写入的字符（0-255 的 `int` 值）
  - 字符已写入 `stdout`

- **Case 2 写入错误**
  - 返回 `EOF`（-1）
  - `stdout` 设置 `F_ERR` 标志

#### 系统算法

```
putchar(c):
  return do_putc(c, stdout)
```

参见 [putc.h.md](./putc.h.md) 中 `do_putc` 的完整算法描述。

#### 不变量

- 若 `stdout` 为行缓冲模式，写入 `\n` 时会触发缓冲区刷出

#### 依赖

- `do_putc(int, FILE *)` — 智能锁字符写入函数（定义于 `putc.h`）
- `stdout` — 标准输出 FILE 指针（`<stdio.h>` 全局变量）
