# ext.c 规约

> musl libc GNU stdio 扩展函数实现。提供 `stdio_ext.h` 中声明的流查询与操作函数。

---

## 依赖图

```
_flushlbf (Public — _GNU_SOURCE)
  └── fflush(0)                              — 刷新所有流 (src/stdio/fflush.c)

__fsetlocking (Public — _GNU_SOURCE)
  └── (无实质依赖 — 直接返回 0)

__fwriting (Public — _GNU_SOURCE)
  └── f->flags & F_NORD, f->wend             — FILE 结构体字段直接访问

__freading (Public — _GNU_SOURCE)
  └── f->flags & F_NOWR, f->rend             — FILE 结构体字段直接访问

__freadable (Public — _GNU_SOURCE)
  └── f->flags & F_NORD                      — FILE 结构体字段直接访问

__fwritable (Public — _GNU_SOURCE)
  └── f->flags & F_NOWR                      — FILE 结构体字段直接访问

__flbf (Public — _GNU_SOURCE)
  └── f->lbf                                 — FILE 结构体字段直接访问

__fbufsize (Public — _GNU_SOURCE)
  └── f->buf_size                            — FILE 结构体字段直接访问

__fpending (Public — _GNU_SOURCE)
  └── f->wend, f->wpos, f->wbase             — FILE 结构体字段直接访问

__fpurge (Public — _GNU_SOURCE)
  └── f->wpos, f->wbase, f->wend, f->rpos, f->rend — FILE 结构体字段直接访问

weak_alias(__fpurge, fpurge)                 — 无下划线公有别名
```

---

## 函数规约

### 1. _flushlbf

```c
void _flushlbf(void);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

刷新所有行缓冲（line-buffered）的 FILE 流。通过调用 `fflush(0)`（或 `fflush(NULL)`），刷新所有打开的输出流，触发所有行缓冲流中未写入的数据实际输出。

#### 前置条件

- 无

#### 后置条件

- 所有打开的行缓冲 FILE 流中挂起（pending）的数据被刷新到底层文件描述符
- 无返回值

#### 系统算法

```
_flushlbf():
  fflush(0)  // 或 fflush(NULL) — 刷新所有打开流
```

#### 依赖

- `fflush(FILE *stream)` — 刷新 FILE 流缓冲区，`NULL` 参数表示刷新所有打开流（定义于 `src/stdio/fflush.c`）

---

### 2. __fsetlocking

```c
int __fsetlocking(FILE *f, int type);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

设置 FILE 流的锁定行为。在 GNU libc 中用于控制隐式锁定的开/关。musl 的实现始终返回 `0`（表示锁定行为未改变），因为 musl 的 FILE 对象不使用可切换的内部锁策略。

**参数 `type`**:
- `FSETLOCKING_INTERNAL` (0) — 设置隐式锁定
- `FSETLOCKING_BYCALLER` (1) — 调用者自行锁定
- `FSETLOCKING_QUERY` (2) — 查询当前状态（在 musl 中始终返回 0）

#### 前置条件

- `f`: 非空 FILE 指针
- `type`: 任意 int 值（musl 中忽略）

#### 后置条件

- 始终返回 `0`（musl 的简化实现）

#### 系统算法

```
__fsetlocking(f, type):
  return 0  // musl: 无操作，始终返回 0
```

---

### 3. __fwriting

```c
int __fwriting(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

查询 FILE 流当前是否处于"正在写入"状态。返回非零值表示流正在被写或有待刷新的写缓冲区数据。检查两个条件：`flags` 中 `F_NORD`（不可读）被设置，或写缓冲区存在挂起数据（`wend != NULL`）。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回非零值：流处于写模式（不可读 或 存在待写数据）
- 返回 `0`：流未处于写模式

#### 系统算法

```
__fwriting(f):
  return (f->flags & F_NORD) || f->wend
```

---

### 4. __freading

```c
int __freading(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

查询 FILE 流当前是否处于"正在读取"状态。返回非零值表示流正在被读或有可用的读缓冲区数据。检查两个条件：`flags` 中 `F_NOWR`（不可写）被设置，或读缓冲区存在可用数据（`rend != NULL`）。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回非零值：流处于读模式（不可写 或 存在可读数据）
- 返回 `0`：流未处于读模式

#### 系统算法

```
__freading(f):
  return (f->flags & F_NOWR) || f->rend
```

---

### 5. __freadable

```c
int __freadable(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

查询 FILE 流是否可读（即 `F_NORD` 标志未被设置）。返回非零值表示流当前允许读取操作。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回非零值：流可读（`F_NORD` 标志未设置）
- 返回 `0`：流不可读

#### 系统算法

```
__freadable(f):
  return !(f->flags & F_NORD)
```

---

### 6. __fwritable

```c
int __fwritable(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

查询 FILE 流是否可写（即 `F_NOWR` 标志未被设置）。返回非零值表示流当前允许写入操作。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回非零值：流可写（`F_NOWR` 标志未设置）
- 返回 `0`：流不可写

#### 系统算法

```
__fwritable(f):
  return !(f->flags & F_NOWR)
```

---

### 7. __flbf

```c
int __flbf(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

查询 FILE 流是否使用行缓冲（line-buffered）模式。返回非零值表示流为行缓冲。检查 `lbf` 字段是否大于等于 `0`（即非 EOF：`lbf` 为 `'\n'` 表示行缓冲，`EOF` 表示全缓冲，`0` 表示无缓冲）。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回非零值：流为行缓冲
- 返回 `0`：流为全缓冲或无缓冲

#### 系统算法

```
__flbf(f):
  return f->lbf >= 0
```

---

### 8. __fbufsize

```c
size_t __fbufsize(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

返回 FILE 流缓冲区的大小（以字节为单位）。直接访问 FILE 结构体的 `buf_size` 字段。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回流的缓冲区大小 `buf_size`（可能为 `0` 表示无缓冲）

#### 系统算法

```
__fbufsize(f):
  return f->buf_size
```

---

### 9. __fpending

```c
size_t __fpending(FILE *f);
```

[Visibility]: User — 声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数

#### Intent

返回 FILE 流写缓冲区中待写入的字节数（pending bytes）。若写缓冲区存在（`wend != NULL`），返回 `wpos - wbase`；否则返回 `0`。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回 `wpos - wbase`（若 `wend` 非空）或 `0`（若无写缓冲区）

#### 系统算法

```
__fpending(f):
  return f->wend ? f->wpos - f->wbase : 0
```

---

### 10. __fpurge / fpurge

```c
int __fpurge(FILE *f);
// weak_alias(__fpurge, fpurge);
```

[Visibility]: User — `fpurge` 是无下划线的公开别名（Solaris/BSD 传统，GNU 扩展），声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。`__fpurge` 为 musl 内部前缀版本

#### Intent

清空 FILE 流的所有内部缓冲区（读和写缓冲区），丢弃所有未读和未写的数据。将 `wpos`、`wbase`、`wend`、`rpos`、`rend` 全部设置为 `NULL`，使流的状态如同刚打开时。

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- **Case 1 清空成功**
  - 读缓冲区被清空（`rpos = rend = NULL`）
  - 写缓冲区被清空（`wpos = wbase = wend = NULL`）
  - 缓冲区中所有未处理数据已丢弃
  - 返回 `0`

#### 系统算法

```
__fpurge(f):
  f->wpos = f->wbase = f->wend = 0  // 清空写缓冲区
  f->rpos = f->rend = 0             // 清空读缓冲区
  return 0
```

#### 不变量

- 清空操作丢弃数据后不可恢复
- `__fpurge` 始终返回 `0`（musl 实现）
- `fpurge` 是无下划线别名（weak_alias），与 `__fpurge` 完全等价

---

## 所有函数共性依赖

| 依赖 | 来源 |
|------|------|
| `FILE` 结构体（`flags`, `wend`, `rend`, `wpos`, `wbase`, `rpos`, `lbf`, `buf_size` 字段） | `src/internal/stdio_impl.h` |
| `F_NORD`, `F_NOWR` — FILE 标志位 | `src/internal/stdio_impl.h` |
| `fflush(FILE *)` | `src/stdio/fflush.c` |
| `weak_alias` 宏 | `src/internal/libc.h` |

---

## 标志位语义速查

| 字段 | 含义 |
|------|------|
| `f->flags & F_NORD` | 流不可读（已进入写模式） |
| `f->flags & F_NOWR` | 流不可写（已进入读模式） |
| `f->flags & F_ERR` | 流出错 |
| `f->flags & F_EOF` | 流遇到文件尾 |
| `f->lbf >= 0` | 行缓冲模式（lbf='\n' 行缓冲，lbf=EOF 全缓冲，lbf=0 无缓冲） |
| `f->wpos - f->wbase` | 写入缓冲区中待写入字节数 |
| `f->rend - f->rpos` | 读缓冲区中剩余可读字节数 |
| `f->buf_size` | 缓冲区总容量 |
