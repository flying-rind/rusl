# ctermid.c 规约

> musl libc POSIX 控制终端路径名获取函数。`ctermid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
ctermid (Public)
  └── strcpy(s, "/dev/tty") — 字符串拷贝（仅当 s != NULL 时）
```

---

## 函数规约

### ctermid

```c
char *ctermid(char *s);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

生成控制终端的路径名字符串。在 Linux/musl 上始终返回字符串 `"/dev/tty"`，因为 `/dev/tty` 是进程控制终端的通用引用路径。

#### 前置条件

- `s`: 可为 `NULL` 或指向至少 `L_ctermid` 字节缓冲区的指针

#### 后置条件

- **Case 1 `s` 不是 `NULL`**
  - 字符串 `"/dev/tty"` 被拷贝到 `s` 指向的缓冲区
  - 返回 `s`

- **Case 2 `s` 是 `NULL`**
  - 直接返回指向字符串字面量 `"/dev/tty"` 的指针
  - 返回的指针指向只读数据段

#### 系统算法

```
ctermid(s):
  if s != NULL:
    return strcpy(s, "/dev/tty")    // 1a. 将 "/dev/tty" 拷贝到用户缓冲区，返回 s
  else:
    return "/dev/tty"               // 1b. 返回只读字符串字面量
```

即：
1. 如果用户提供了缓冲区（`s != NULL`），使用 `strcpy` 将 `/dev/tty` 拷贝到用户缓冲区，返回 `s`
2. 如果用户未提供缓冲区（`s == NULL`），直接返回指向字符串字面量 `"/dev/tty"` 的指针

注意：musl 实现中 `"/dev/tty"` 是硬编码的字符串（8 字节 + null 终止符），这是一个满足 POSIX 标准的合法实现。POSIX 标准保证 `/dev/tty` 在所有符合标准的系统上都是控制终端的同义名。

#### 依赖

- `strcpy` — 字符串拷贝（`src/string/strcpy.c`）
- `L_ctermid` — 控制终端路径名最大长度宏（通常为 9），定义在 `<stdio.h>`
