# setlinebuf.c 规约

> musl libc `setlinebuf` 实现 — GNU 扩展，将 FILE 流设为行缓冲模式。不提供用户缓冲区，musl 将根据实际使用情况自动分配内部缓冲区。

---

## 依赖图

```
setlinebuf (Public, GNU extension)
  └── setvbuf (see setvbuf.c spec)
```

---

## 函数规约

### 1. setlinebuf

```c
void setlinebuf(FILE *f);
```

[Visibility]: User — GNU / BSD 扩展函数（需定义 `_GNU_SOURCE` 或 `_BSD_SOURCE`），用户程序可直接调用

#### Intent

将流 `f` 设为行缓冲模式。等价于 `setvbuf(f, 0, _IOLBF, 0)`：
- 不提供用户缓冲区（buf = NULL），让 musl 在需要读/写时自动分配内部缓冲区
- 设置为行缓冲模式：每当遇到换行符 `'\n'` 时自动刷新输出缓冲区

#### 前置条件

- `f`: 有效的 `FILE *` 指针，指向已打开的流
- 根据 C 标准和实现惯例，`setlinebuf` 的行为仅在流打开后、任何读写操作前被调用时是定义良好的
- 编译时需定义 `_GNU_SOURCE` 或 `_BSD_SOURCE` 以获取函数声明

#### 后置条件

- 流 `f` 的缓冲模式已设置为行缓冲（`_IOLBF`）
- 流 `f` 的 `F_SVB` 标志被置位
- 无返回值（`void`）

#### 依赖

- `setvbuf(FILE *, const char *, int, size_t)` — 内部依赖，见 `setvbuf.c` spec

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `_IOLBF` | 1 | `<stdio.h>` | 行缓冲模式 |
