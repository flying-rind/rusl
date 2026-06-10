# setbuffer.c 规约

> musl libc `setbuffer` 实现 — GNU 扩展，为 FILE 流设置缓冲模式和自定义大小缓冲区。与 `setbuf` 的区别在于允许调用方指定缓冲区大小。

---

## 依赖图

```
setbuffer (Public, GNU extension)
  └── setvbuf (see setvbuf.c spec)
```

---

## 函数规约

### 1. setbuffer

```c
void setbuffer(FILE *f, char *buf, size_t size);
```

[Visibility]: User — GNU / BSD 扩展函数（需定义 `_GNU_SOURCE` 或 `_BSD_SOURCE`），用户程序可直接调用

#### Intent

带自定义缓冲区大小的缓冲设置接口。等价于 `setvbuf(f, buf, buf ? _IOFBF : _IONBF, size)`：
- 若 `buf` 非 NULL，将流 `f` 设为全缓冲模式，缓冲区大小为 `size`
- 若 `buf` 为 NULL，将流 `f` 设为无缓冲模式（此时 `size` 参数被忽略）

#### 前置条件

- `f`: 有效的 `FILE *` 指针，指向已打开的流
- `buf`: 若非 NULL，指向调用方提供的缓冲区，大小至少为 `size` 字节。该缓冲区在流关闭前必须保持有效
- `size`: 若 `buf` 非 NULL，指定缓冲区大小（字节数）
- 根据 C 标准和实现惯例，`setbuffer` 的行为仅在流打开后、任何读写操作前被调用时是定义良好的
- 编译时需定义 `_GNU_SOURCE` 或 `_BSD_SOURCE` 以获取函数声明

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
