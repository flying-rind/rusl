# setbuf.c 规约

> musl libc `setbuf` 实现 — 为 FILE 流设置缓冲模式和缓冲区。`setbuf` 是 `setvbuf` 的简化包装：当 `buf` 为 NULL 时设为无缓冲模式，否则设为全缓冲模式并使用 BUFSIZ 作为缓冲区大小。

---

## 依赖图

```
setbuf (Public)
  └── setvbuf (see setvbuf.c spec)
```

---

## 函数规约

### 1. setbuf

```c
void setbuf(FILE *restrict f, char *restrict buf);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

简化缓冲区设置接口。等价于 `setvbuf(f, buf, buf ? _IOFBF : _IONBF, BUFSIZ)`：
- 若 `buf` 非 NULL，将流 `f` 设为全缓冲模式，缓冲区大小为 `BUFSIZ`（1024）
- 若 `buf` 为 NULL，将流 `f` 设为无缓冲模式

#### 前置条件

- `f`: 有效的 `FILE *` 指针，指向已打开的流
- `buf`: 若非 NULL，指向调用方提供的缓冲区，大小至少为 `BUFSIZ` 字节。该缓冲区在流关闭前必须保持有效
- 根据 C 标准，`setbuf` 的行为仅在流打开后、任何读写操作前被调用时是定义良好的

#### 后置条件

- 流 `f` 的缓冲模式已设置：全缓冲（buf 非 NULL）或无缓冲（buf 为 NULL）
- 流 `f` 的 `F_SVB` 标志被置位
- 无返回值（`void`）

#### 依赖

- `setvbuf(FILE *, const char *, int, size_t)` — 内部依赖，见 `setvbuf.c` spec

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `_IOFBF` | 0 | `<stdio.h>` | 全缓冲模式 |
| `_IONBF` | 2 | `<stdio.h>` | 无缓冲模式 |
| `BUFSIZ` | 1024 | `<stdio.h>` | 默认缓冲区大小 |
