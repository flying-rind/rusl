# fsetpos.c 规约

> musl libc 文件位置设置实现（ISO C 标准接口）。将文件流位置恢复到先前由 `fgetpos` 保存的位置。

---

## 依赖图

```
fsetpos
  └─> __fseeko                   (see fseek.c spec)
```

---

## 数据结构分析

### `fpos_t` 类型

- musl 中 `fpos_t` 定义为 `long long`
- `fsetpos` 从 `fpos_t` 读取 `long long` 值作为绝对文件偏移（`SEEK_SET`）

---

## 函数规约

### 1. fsetpos

```c
int fsetpos(FILE *f, const fpos_t *pos);
```

[Visibility]: User — 标准 C 库函数（ISO C），声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

将文件流定位到 `*pos` 所表示的位置。用于配合 `fgetpos` 实现位置保存/恢复。内部将 `fpos_t` 值解释为绝对偏移量，以 `SEEK_SET` 为基准调用 `__fseeko`。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`
- `pos`: 非 NULL 的 `const fpos_t*`，指向由合法 `fgetpos` 调用填充的 `fpos_t` 值
- `*pos` 中的值应对应于文件的有效偏移量
- `f` 的底层定位操作可用（文件可定位，如常规文件，不能是管道或终端）

#### 后置条件

**Case 1: 成功**
- 文件位置指示符被设置为 `*(const long long *)pos` 所表示的位置
- 写缓冲区已刷写，读缓冲区被丢弃（由 `__fseeko` 内部完成）
- `F_EOF` 标志被清除
- 返回 `0`

**Case 2: 失败**
- `__fseeko(f, *(const long long *)pos, SEEK_SET)` 返回 `-1`
- 返回 `-1`
- errno 由 `__fseeko` 设置（可能包括 `EINVAL`、`ESPIPE` 等）
- `*pos` 不被修改（`const` 参数）

#### 系统算法

```
fsetpos(f, pos):
  return __fseeko(f, *(const long long *)pos, SEEK_SET)
```

#### 不变量

- `*pos` 不被修改（`const` 语义保证）
- `fpos_t` 中存储的值被解释为从文件起始的绝对偏移量（`SEEK_SET`）

#### 依赖

- `__fseeko` — 加锁定位操作（定义于 `src/stdio/fseek.c`，见 `fseek.c` spec）
- `SEEK_SET` — 从文件起始定位（`stdio.h`）
