# getchar.c 规约

> musl libc 标准输入单字符读取实现。从 `stdin` 读取一个字符。

---

## 依赖图

```
getchar (Public)
  └── do_getc (inline, from "getc.h")
        ├── getc_unlocked (宏)
        ├── locking_getc (static)
        └── __pthread_self

stdin (全局变量, 来自 <stdio.h>)
```

---

## 函数规约

### getchar

```c
int getchar(void);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

从标准输入流 `stdin` 读取一个字符。等价于 `getc(stdin)` 或 `fgetc(stdin)`。

#### 前置条件

- `stdin` 已正确初始化并处于可读状态

#### 后置条件

- **Case 1 成功读取字符**
  - 返回读取到的字符（0-255 的 `int` 值）
  - `stdin` 流位置前进一个字符

- **Case 2 到达文件末尾**
  - 返回 `EOF`（-1）
  - `stdin` 设置 `F_EOF` 标志

- **Case 3 读取错误**
  - 返回 `EOF`
  - `stdin` 设置 `F_ERR` 标志

#### 系统算法

```
getchar():
  return do_getc(stdin)
```

参见 [getc.h.md](./getc.h.md) 中 `do_getc` 的完整算法描述。

#### 依赖

- `do_getc(FILE *)` — 智能锁字符读取函数（定义于 `getc.h`）
- `stdin` — 标准输入 FILE 指针（`<stdio.h>` 全局变量）
