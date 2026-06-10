# getc.c 规约

> musl libc 标准 IO 宏兼容字符读取实现。`getc` 在 `<stdio.h>` 中通常被定义为宏以提供性能优化，但同时也需要作为函数存在以支持函数指针等场景。musl 提供 `getc` 函数实现和 `_IO_getc` 弱别名。

---

## 依赖图

```
getc (Public)
  └── do_getc (inline, from "getc.h")
        ├── getc_unlocked (宏)
        ├── locking_getc (static)
        └── __pthread_self

weak_alias(getc, _IO_getc)
```

---

## 函数规约

### 1. getc

```c
int getc(FILE *f);
```

[Visibility]: User — `<stdio.h>` 标准库函数（宏的备选函数实现），用户程序可调用

#### Intent

从 FILE 流 `f` 中读取一个字符。通常 `<stdio.h>` 以宏形式内联展开为高效实现，但 musl 同时提供函数实现以支持：
- 通过函数指针调用 `getc`
- `#undef getc` 后使用真实函数
- `_IO_getc` 别名引用（libstdc++ 等使用）

#### 前置条件

- `f`: 非空 FILE 指针，指向已打开的读模式流

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
getc(f):
  return do_getc(f)
```

参见 [getc.h.md](./getc.h.md) 中 `do_getc` 的完整算法描述。

#### 依赖

- `do_getc(FILE *)` — `getc.h` 中定义的 inline 函数

---

### 2. _IO_getc (weak_alias)

```c
// weak_alias(getc, _IO_getc);
int _IO_getc(FILE *f);
```

[Visibility]: Internal — 传统 `_IO_` 前缀历史兼容别名，供 glibc 兼容代码使用

前置/后置条件及行为：完全等同于 `getc`。
