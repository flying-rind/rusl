# ext2.c 规约

> musl libc GNU stdio 扩展函数实现（第二部分）。提供 `stdio_ext.h` 中声明的读缓冲区窥探函数和错误标志设置函数。

---

## 依赖图

```
__freadahead (Public — _GNU_SOURCE)
  └── f->rend, f->rpos               — FILE 结构体字段直接访问

__freadptr (Public — _GNU_SOURCE)
  └── f->rpos, f->rend               — FILE 结构体字段直接访问

__freadptrinc (Public — _GNU_SOURCE)
  └── f->rpos                        — FILE 结构体字段直接访问

__fseterr (Public — _GNU_SOURCE)
  └── f->flags |= F_ERR              — FILE 结构体字段直接访问
```

---

## 函数规约

### 1. __freadahead

```c
size_t __freadahead(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

返回 FILE 流读缓冲区中还可读取的字节数（预读字节数）。计算方式为 `rend - rpos`（读缓冲区末尾指针减去当前位置指针）。若读缓冲区为空或不存在（`rend == 0`），返回 `0`。

**用途**: 允许调用者在实际调用 `fread`/`fgetc` 前窥探缓冲区中有多少数据可用。配合 `__freadptr` 可实现零拷贝读取。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回读缓冲区中剩余可读字节数（可能为 `0`）

#### 系统算法

```
__freadahead(f):
  return f->rend ? f->rend - f->rpos : 0
```

---

### 2. __freadptr

```c
const char *__freadptr(FILE *f, size_t *sizep);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

返回指向 FILE 流读缓冲区当前位置的指针，并通过 `*sizep` 返回可读字节数。若读缓冲区为空（`rpos == rend`），返回 `NULL` 且不修改 `*sizep`。

**用途**: 允许调用者直接访问 FILE 内部缓冲区，实现零拷贝读取。配合 `__freadptrinc` 来推进读指针。

#### 前置条件

- `f`: 非空 FILE 指针
- `sizep`: 非空指针，指向 `size_t` 变量

#### 后置条件

- **Case 1 读缓冲区有数据**
  - `*sizep` 被设置为 `f->rend - f->rpos`
  - 返回指向 `f->rpos` 的 `const char*` 指针

- **Case 2 读缓冲区无可用数据**
  - `*sizep` 不变
  - 返回 `NULL`

#### 系统算法

```
__freadptr(f, sizep):
  if f->rpos == f->rend:        // 缓冲区空或被清空
    return 0
  *sizep = f->rend - f->rpos    // 设置可用字节数
  return (const char *)f->rpos  // 返回当前位置指针
```

---

### 3. __freadptrinc

```c
void __freadptrinc(FILE *f, size_t inc);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

将 FILE 流读缓冲区的读指针推进 `inc` 字节，表示这些数据已被调用者消费。与 `__freadptr` 配合使用实现零拷贝读取模式：先用 `__freadptr` 获取缓冲区指针，直接访问数据，再用 `__freadptrinc` 标记已消费。

#### 前置条件

- `f`: 非空 FILE 指针
- `inc`: 推进量，应不大于 `__freadahead(f)` 返回的可用字节数（调用者有责任确保此条件，musl 不检查）

#### 后置条件

- `f->rpos` 增加 `inc` 字节
- 已推进的数据不再可通过 `__freadptr` 访问

#### 系统算法

```
__freadptrinc(f, inc):
  f->rpos += inc
```

#### 注意事项

- musl 实现不做边界检查，`inc` 超出 `rend - rpos` 会导致读指针越过读缓冲区末尾，后续行为依赖于底层实现

---

### 4. __fseterr

```c
void __fseterr(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

直接设置 FILE 流的错误标志位（`F_ERR`）。与 `ferror(f)`（查询）互补，允许调用者手动将流标记为错误状态。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- `f->flags` 的 `F_ERR` 位被设置
- 之后 `ferror(f)` 将返回非零值，后续 I/O 操作将返回错误

#### 系统算法

```
__fseterr(f):
  f->flags |= F_ERR
```

---

## 所有函数共性依赖

| 依赖 | 来源 |
|------|------|
| `FILE` 结构体（`flags`, `rend`, `rpos` 字段） | `src/internal/stdio_impl.h` |
| `F_ERR` — 错误标志位 | `src/internal/stdio_impl.h` |
| `<stdio_ext.h>` — 接口声明 | 系统头文件 |

---

## 零拷贝读取模式示例

```
// 传统读取(fread — 有拷贝):
char buf[1024];
size_t n = fread(buf, 1, sizeof buf, f);

// 零拷贝读取(__freadptr + __freadptrinc):
size_t n;
const char *p = __freadptr(f, &n);
process_data(p, n);             // 直接访问 FILE 内部缓冲区
__freadptrinc(f, n);            // 推进读指针
```

**关键规则**:
- 在调用 `__freadptr` 和 `__freadptrinc` 之间不得对同一流调用任何其他 stdio 函数（可能使返回的指针失效）
- `__freadptrinc` 的 `inc` 不得超出 `__freadahead` 返回的可用字节数
- `__freadptr` 返回的指针仅在流未被进一步操作时有效
