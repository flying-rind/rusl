# getchar_unlocked.c 规约

> musl libc 免锁标准输入单字符读取实现。从 `stdin` 读取一个字符，不获取流锁。

---

## 依赖图

```
getchar_unlocked (Public)
  └── getc_unlocked(stdin)  (see getc_unlocked.c / stdio_impl.h)
        └── __uflow (see __uflow.c)

stdin (全局变量, 来自 <stdio.h>)
```

---

## 函数规约

### 1. getchar_unlocked

```c
int getchar_unlocked(void);
```

[Visibility]: User — `<stdio.h>` POSIX 免锁扩展，用户程序可直接调用

#### Intent

从标准输入流 `stdin` 读取一个字符，不获取流锁。等价于 `getc_unlocked(stdin)`。

**关键差异**：与 `getchar()` 不同，此函数不调用 `FLOCK(stdin)`/`FUNLOCK(stdin)`。调用者必须确保在调用此函数前已自行获取 `stdin` 的锁，或在单线程环境下使用。配合 `flockfile(stdin)`/`funlockfile(stdin)` 使用。

#### 前置条件

- `stdin` 已正确初始化并处于可读状态
- 调用者已获取 `stdin` 的锁（多线程环境），或确定当前为单线程访问

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
getchar_unlocked():
  return getc_unlocked(stdin)
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `getc_unlocked(FILE *)` — 无锁字符读取（来自 `stdio_impl.h` 宏，最终委托 `__uflow`）
- `stdin` — 标准输入 FILE 指针（`<stdio.h>` 全局变量）
