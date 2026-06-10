# getc_unlocked.c 规约

> musl libc 免锁 FILE 流单字符读取的函数实现。`getc_unlocked` 在 `<stdio.h>` 中通常被定义为宏以提供内联优化，但 musl 同时提供函数实现以支持函数指针等场景。

---

## 依赖图

```
getc_unlocked (Public)
  └── getc_unlocked 宏 (来自 stdio_impl.h)
        └── __uflow (see __uflow.c)

fgetc_unlocked (weak_alias) ──> getc_unlocked
_IO_getc_unlocked (weak_alias) ──> getc_unlocked
```

---

## 函数规约

### 1. getc_unlocked

```c
int (getc_unlocked)(FILE *f);
```

[Visibility]: User — `<stdio.h>` POSIX 免锁扩展，用户程序可直接调用。函数名带有括号以避免宏展开，确保链接到函数实现而非宏。

#### Intent

从 FILE 流 `f` 中读取一个字符，不获取流锁。

**关键差异**：与 `getc(f)` 不同，此函数直接调用 `getc_unlocked` 宏（内部指向 `__uflow`），不执行 `do_getc` 的加锁逻辑。调用者必须确保在调用此函数前已自行获取 `f` 的锁，或在单线程环境下使用。

#### 前置条件

- `f`: 非空 FILE 指针，指向已打开的读模式流
- 调用者已获取 `f` 的锁（多线程环境），或确定当前为单线程访问
- `f` 已通过 `__toread` 初始化为读模式

#### 后置条件

- **Case 1 成功读取字符**
  - 返回读取到的字符（0-255 的 `int` 值）
  - FILE 流位置前进一个字符

- **Case 2 到达文件末尾**
  - 返回 `EOF`（-1）
  - FILE 流设置 `F_EOF` 标志

- **Case 3 读取错误**
  - 返回 `EOF`
  - FILE 流设置 `F_ERR` 标志

#### 系统算法

```
getc_unlocked(f):
  return getc_unlocked(f)  // 调用 stdio_impl.h 中的宏，委托给 __uflow
```

#### 不变量

- 不执行加锁操作（调用者负责锁管理）

#### 依赖

- `getc_unlocked` 宏 — 定义于 `stdio_impl.h`，内部调用 `__uflow`
- `__uflow(FILE *)` — 无锁底层读取引擎（见 `__uflow.c`）

---

### 2. fgetc_unlocked / _IO_getc_unlocked (weak_alias)

```c
weak_alias(getc_unlocked, fgetc_unlocked);
weak_alias(getc_unlocked, _IO_getc_unlocked);
```

[Visibility]: User — `fgetc_unlocked` 为 POSIX 标准名称，`_IO_getc_unlocked` 为 glibc 兼容别名

- **Intention**: 提供 POSIX 标准名称 `fgetc_unlocked` 和 glibc 兼容名称 `_IO_getc_unlocked`。与 `getc_unlocked` 行为完全相同。

前置/后置条件及行为：完全等同于 `getc_unlocked`。
