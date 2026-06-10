# \_\_fclose_ca.c 规约

> musl libc 内部调用方分配 FILE（Caller-Allocated）的关闭操作实现。为调用方分配的 `FILE` 结构体提供关闭语义：仅调用 `f->close` 函数指针，不释放 `FILE` 内存。

---

## 依赖图

```
__fclose_ca
  └─> f->close(f)    (see __stdio_close.c spec)
```

---

## 函数规约

### 1. \_\_fclose_ca

```c
int __fclose_ca(FILE *f);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。与 `__fopen_rb_ca` 配套使用，供调用方自行管理 `FILE` 结构体内存的场景。

#### Intent

关闭调用方分配的 `FILE` 流。与标准 `fclose` 的关键区别：
- **不释放 `FILE*` 内存**（`f` 由调用方管理）
- 仅调用 `f->close(f)` 关闭底层文件描述符

#### 前置条件

- `f`: `FILE*`，其 `close` 函数指针已设置为有效的关闭函数（如 `__stdio_close`）
- `f` 指向的内存由调用方管理（不在堆上，或由调用方自行释放）

#### 后置条件

**Case 1: 关闭成功**

- 底层文件描述符已关闭
- `FILE*` 内存未被释放（调用方负责后续处理）
- 返回 `f->close(f)` 的返回值（通常为 `0`）

**Case 2: 关闭失败**

- 返回 `-1`，errno 由底层关闭函数设置

#### 系统算法

```
__fclose_ca(f):
  return f->close(f)
```

#### 不变量

- 仅转发调用，不执行内存释放
- `f` 指针在调用后仍有效（由调用方管理其生命周期）

#### 依赖

- `f->close()` — 流关闭函数指针（通常为 `__stdio_close`，本模块）
