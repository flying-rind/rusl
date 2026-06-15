# posix_close.c 规约

> musl libc POSIX 关闭文件描述符函数。`posix_close` 在 `<unistd.h>` 中声明（POSIX 标准）。

---

## 依赖图

```
posix_close (Public)
  └── close(fd) — 委托给标准 close 函数，忽略 flags 参数
```

---

## 函数规约

### posix_close

```c
int posix_close(int fd, int flags);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

POSIX 标准文件描述符关闭函数。与 `close` 的区别在于支持 `flags` 参数。当前 musl 实现将 `flags` 忽略，直接委托给 `close`。

#### 前置条件

- `fd`: 有效的已打开文件描述符
- `flags`: 当前唯一有效值为 `POSIX_CLOSE_RESTART` (0)

#### 后置条件

- 等同于 `close(fd)` 的后置条件
- 返回 0 表示成功，返回 -1 表示错误

#### 系统算法

```
posix_close(fd, flags):
  return close(fd)                // 直接委托给 close
```

#### 依赖

- `close(int fd)` — 标准 close 函数
