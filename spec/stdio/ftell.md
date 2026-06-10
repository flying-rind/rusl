# ftell.c 规约

> musl libc 文件流当前位置查询实现。提供 `ftell`、`ftello` 以及内部不加锁版本 `__ftello` 和 `__ftello_unlocked`。

---

## 依赖图

```
ftell
  └─> __ftello                   (see ftell.c spec)
        ├─> FLOCK                (see stdio_impl.h / __lockfile.c spec)
        │     └─> __lockfile     (see __lockfile.c spec)
        ├─> __ftello_unlocked    (see ftell.c spec)
        │     └─> f->seek        (function pointer, default: __stdio_seek)
        │         └─> __lseek    (see <unistd.h>)
        └─> FUNLOCK              (see stdio_impl.h / __lockfile.c spec)
              └─> __unlockfile   (see __lockfile.c spec)

ftello = weak_alias(__ftello)
```

---

## 数据结构分析

### FILE 结构体关键字段

| 字段 | 类型 | 用途 |
|------|------|------|
| `f->flags` | `unsigned` | 文件流状态标志（含 `F_APP` 追加模式） |
| `f->rpos`, `f->rend` | `unsigned char *` | 读缓冲区指针 |
| `f->wpos`, `f->wbase` | `unsigned char *` | 写缓冲区指针 |
| `f->seek` | `off_t (*)(FILE *, off_t, int)` | 底层定位函数指针 |

### ftell 特殊行为

- `ftell` 返回 `long` 类型，但内部使用 `off_t`（通常为 `long long`）计算
- 当 `off_t` 计算结果超出 `LONG_MAX` 时，返回错误

---

## 函数规约

### 1. \_\_ftello_unlocked

```c
off_t __ftello_unlocked(FILE *f);
```

[Visibility]: Internal (hidden) — musl 内部实现，不对外暴露。由不加锁上下文调用以在不加锁情况下获取文件位置。

#### Intent

返回指定 `FILE*` 流的当前逻辑位置（从文件起始的字节偏移）。通过底层 `f->seek` 获取内核文件偏移量，再根据缓冲区状态补偿未刷写或已缓冲的数据量。区分读缓冲区和写缓冲区的处理：

- **读缓冲区**: 内核位置超前于逻辑位置（预读了数据），需减去 `rpos` 与 `rend` 之间的剩余量
- **写缓冲区**: 逻辑位置超前于内核位置（写入了未刷新的数据），需加上 `wpos` 与 `wbase` 之间的等待刷写量

特别地：追加模式（`F_APP`）下若写缓冲区非空，使用 `SEEK_END` 而非 `SEEK_CUR` 作为定位基准，确保获取正确的末尾位置。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`，`f->seek` 函数指针有效
- 调用方已持有 `f` 的锁（或不需锁的场景）

#### 后置条件

**Case 1: 成功**
- 返回当前逻辑文件位置（`off_t`，非负值）

**Case 2: 失败 — 底层 seek 失败**
- 返回 `-1`（`(off_t)-1`）
- errno 由底层 `f->seek` 设置

#### 系统算法

```
__ftello_unlocked(f):
  1. 获取内核文件偏移量
     base_whence = SEEK_CUR
     if (f->flags & F_APP) and (f->wpos != f->wbase):
       base_whence = SEEK_END      // 追加模式+有未刷写数据：用 SEEK_END
     pos = f->seek(f, 0, base_whence)
     if pos < 0: return pos

  2. 缓冲区偏移补偿
     if f->rend:                   // 有读缓冲区数据
       pos += f->rpos - f->rend    // 减去预读但未消费的数据量
     else if f->wbase:             // 有写缓冲区数据
       pos += f->wpos - f->wbase   // 加上已写但未刷写的数据量

  3. return pos
```

#### 不变量

- 不修改 `f` 的任何字段（纯查询操作）
- 返回的逻辑位置在成功时始终 >= 0

#### 依赖

- `f->seek` — 底层定位函数指针（默认 `__stdio_seek` → `__lseek` 系统调用）
- `F_APP` — 追加模式标志（`stdio_impl.h`）

---

### 2. \_\_ftello

```c
off_t __ftello(FILE *f);
```

[Visibility]: Internal (hidden) — 加锁版本的位置查询函数。是 `ftello` 的主实现（弱别名）。

#### Intent

获取文件流锁，查询位置，释放锁。

#### 前置条件

- 同 `__ftello_unlocked`
- 调用方不持有 `f` 的锁

#### 后置条件

- 与 `__ftello_unlocked` 相同

#### 系统算法

```
__ftello(f):
  FLOCK(f)
  pos = __ftello_unlocked(f)
  FUNLOCK(f)
  return pos
```

#### 依赖

- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏
- `__ftello_unlocked` — 不加锁位置查询（同文件）

---

### 3. ftell

```c
long ftell(FILE *f);
```

[Visibility]: User — 标准 C 库函数（ISO C / POSIX），声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

返回流的当前逻辑文件位置，返回类型为 `long`。当内部 `off_t`（可能为 `long long`）结果超出 `LONG_MAX` 时，设置 `EOVERFLOW` 并返回 `-1`。

#### 前置条件

- 同 `__ftello`

#### 后置条件

**Case 1: 成功**
- `__ftello(f)` 返回的非负值在 `LONG_MAX` 范围内
- 返回该位置值（类型 `long`）

**Case 2: 失败 — 位置超出 long 范围**
- `pos > LONG_MAX`
- `errno` 设置为 `EOVERFLOW`
- 返回 `-1`

**Case 3: 失败 — 底层 seek 失败**
- `__ftello(f)` 返回 `-1`
- 返回 `-1`
- errno 由底层设置

#### 系统算法

```
ftell(f):
  pos = __ftello(f)              // off_t 类型
  if pos > LONG_MAX:
    errno = EOVERFLOW
    return -1
  return pos                      // 隐式转换为 long
```

#### 依赖

- `__ftello` — 加锁位置查询（同文件）
- `<limits.h>` — `LONG_MAX`
- `<errno.h>` — `EOVERFLOW`

---

### 4. ftello (weak_alias)

```c
weak_alias(__ftello, ftello);
```

[Visibility]: User — POSIX 标准函数，声明于 `<stdio.h>`（需 `_POSIX_C_SOURCE >= 200112L`）。

- **Intention**: 与 `__ftello` 共享同一实现（返回 `off_t` 类型，支持大文件）。前置/后置条件完全等同于 `__ftello`。
