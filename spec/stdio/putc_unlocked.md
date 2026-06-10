# putc_unlocked.c 规约

> musl libc 免锁 FILE 流单字符写入的函数实现。`putc_unlocked` 在 `<stdio.h>` 中通常被定义为宏以提供内联优化，但 musl 同时提供函数实现以支持函数指针等场景。

---

## 依赖图

```
putc_unlocked (Public)
  └── putc_unlocked 宏 (来自 stdio_impl.h)
        └── __overflow (see __overflow.c)

fputc_unlocked (weak_alias) ──> putc_unlocked
_IO_putc_unlocked (weak_alias) ──> putc_unlocked
```

---

## 函数规约

### 1. putc_unlocked

```c
int (putc_unlocked)(int c, FILE *f);
```

[Visibility]: User — `<stdio.h>` POSIX 免锁扩展，用户程序可直接调用。函数名带有括号以避免宏展开，确保链接到函数实现而非宏。

#### Intent

将字符 `c` 写入 FILE 流 `f`，不获取流锁。

**关键差异**：与 `putc(c, f)` 不同，此函数直接调用 `putc_unlocked` 宏（内部指向 `__overflow`），不执行 `do_putc` 的加锁逻辑。调用者必须确保在调用此函数前已自行获取 `f` 的锁，或在单线程环境下使用。

#### 前置条件

- `c`: 要写入的字符（以 `int` 传递，内部转为 `unsigned char`）
- `f`: 非空 FILE 指针，指向已打开的写模式流
- 调用者已获取 `f` 的锁（多线程环境），或确定当前为单线程访问
- `f` 已通过 `__towrite` 初始化为写模式

#### 后置条件

- **Case 1 成功写入**
  - 返回写入的字符（0-255 的 `int` 值）
  - 字符已写入流缓冲区

- **Case 2 写入错误**
  - 返回 `EOF`（-1）
  - FILE 流设置 `F_ERR` 标志

#### 系统算法

```
putc_unlocked(c, f):
  return putc_unlocked(c, f)  // 调用 stdio_impl.h 中的宏，委托给 __overflow
```

#### 不变量

- 不执行加锁操作（调用者负责锁管理）

#### 依赖

- `putc_unlocked` 宏 — 定义于 `stdio_impl.h`，内部调用 `__overflow`
- `__overflow(FILE *, int)` — 无锁底层写入引擎（见 `__overflow.c`）

---

### 2. fputc_unlocked / _IO_putc_unlocked (weak_alias)

```c
weak_alias(putc_unlocked, fputc_unlocked);
weak_alias(putc_unlocked, _IO_putc_unlocked);
```

[Visibility]: User — `fputc_unlocked` 为 POSIX 标准名称，`_IO_putc_unlocked` 为 glibc 兼容别名

- **Intention**: 提供 POSIX 标准名称 `fputc_unlocked` 和 glibc 兼容名称 `_IO_putc_unlocked`。与 `putc_unlocked` 行为完全相同。

前置/后置条件及行为：完全等同于 `putc_unlocked`。
