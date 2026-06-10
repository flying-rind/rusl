# putc.c 规约

> musl libc 标准 IO 宏兼容字符写入实现。`putc` 在 `<stdio.h>` 中通常被定义为宏以提供性能优化，但同时也需要作为函数存在以支持函数指针等场景。musl 提供 `putc` 函数实现和 `_IO_putc` 弱别名。

---

## 依赖图

```
putc (Public)
  └── do_putc (inline, from "putc.h")
        ├── putc_unlocked (宏)
        ├── locking_putc (static)
        └── __pthread_self

weak_alias(putc, _IO_putc)
```

---

## 函数规约

### 1. putc

```c
int putc(int c, FILE *f);
```

[Visibility]: User — `<stdio.h>` 标准库函数（宏的备选函数实现），用户程序可调用

#### Intent

将字符 `c` 写入 FILE 流 `f`。通常 `<stdio.h>` 以宏形式内联展开为高效实现，但 musl 同时提供函数实现以支持函数指针等用例。

#### 前置条件

- `c`: 要写入的字符（以 `int` 传递，内部转为 `unsigned char`）
- `f`: 非空 FILE 指针，指向已打开的写模式流

#### 后置条件

- **Case 1 成功写入**
  - 返回写入的字符（0-255 的 `int` 值）
  - 字符已写入流

- **Case 2 写入错误**
  - 返回 `EOF`（-1）
  - FILE 流设置 `F_ERR` 标志

#### 系统算法

```
putc(c, f):
  return do_putc(c, f)
```

参见 [putc.h.md](./putc.h.md) 中 `do_putc` 的完整算法描述。

#### 不变量

- 对于行缓冲流，遇到 `\n` 时缓冲区会被刷出

#### 依赖

- `do_putc(int, FILE *)` — `putc.h` 中定义的 inline 函数

---

### 2. _IO_putc (weak_alias)

```c
// weak_alias(putc, _IO_putc);
int _IO_putc(int c, FILE *f);
```

[Visibility]: Internal — 传统 `_IO_` 前缀历史兼容别名

前置/后置条件及行为：完全等同于 `putc`。
