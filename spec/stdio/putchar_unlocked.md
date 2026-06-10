# putchar_unlocked.c 规约

> musl libc 免锁标准输出单字符写入实现。将一个字符写入 `stdout`，不获取流锁。

---

## 依赖图

```
putchar_unlocked (Public)
  └── putc_unlocked(c, stdout)  (see putc_unlocked.c / stdio_impl.h)
        └── __overflow (see __overflow.c)

stdout (全局变量, 来自 <stdio.h>)
```

---

## 函数规约

### 1. putchar_unlocked

```c
int putchar_unlocked(int c);
```

[Visibility]: User — `<stdio.h>` POSIX 免锁扩展，用户程序可直接调用

#### Intent

将字符 `c` 写入标准输出流 `stdout`，不获取流锁。等价于 `putc_unlocked(c, stdout)`。

**关键差异**：与 `putchar()` 不同，此函数不调用 `FLOCK(stdout)`/`FUNLOCK(stdout)`。调用者必须确保在调用此函数前已自行获取 `stdout` 的锁，或在单线程环境下使用。配合 `flockfile(stdout)`/`funlockfile(stdout)` 使用。

#### 前置条件

- `c`: 要写入的字符（以 `int` 传递，内部转为 `unsigned char`）
- `stdout` 已正确初始化并处于可写状态
- 调用者已获取 `stdout` 的锁（多线程环境），或确定当前为单线程访问

#### 后置条件

- **Case 1 成功写入**
  - 返回写入的字符（0-255 的 `int` 值）
  - 字符已写入 `stdout`

- **Case 2 写入错误**
  - 返回 `EOF`（-1）
  - `stdout` 设置 `F_ERR` 标志

#### 系统算法

```
putchar_unlocked(c):
  return putc_unlocked(c, stdout)
```

#### 不变量

- 若 `stdout` 为行缓冲模式，写入 `\n` 时会触发缓冲区刷出

#### 依赖

- `putc_unlocked(int, FILE *)` — 无锁字符写入（来自 `stdio_impl.h` 宏，最终委托 `__overflow`）
- `stdout` — 标准输出 FILE 指针（`<stdio.h>` 全局变量）
