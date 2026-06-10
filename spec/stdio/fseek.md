# fseek.c 规约

> musl libc 文件流定位操作实现。提供 `fseek`、`fseeko` 以及内部不加锁版本 `__fseeko` 和 `__fseeko_unlocked`。

---

## 依赖图

```
fseek
  └─> __fseeko                    (see fseek.c spec)
        ├─> FLOCK                 (see stdio_impl.h / __lockfile.c spec)
        │     └─> __lockfile      (see __lockfile.c spec)
        ├─> __fseeko_unlocked     (see fseek.c spec)
        │     ├─> f->write        (function pointer, set by fdopen/fopen)
        │     └─> f->seek         (function pointer, default: __stdio_seek)
        └─> FUNLOCK               (see stdio_impl.h / __lockfile.c spec)
              └─> __unlockfile    (see __lockfile.c spec)

fseeko = weak_alias(__fseeko)
```

---

## 数据结构分析

### `whence` 参数合法值

| 常量 | 值 | 含义 |
|------|-----|------|
| `SEEK_SET` | `0` | 从文件起始定位 |
| `SEEK_CUR` | `1` | 从当前文件位置定位 |
| `SEEK_END` | `2` | 从文件末尾定位 |

### FILE 结构体关键字段

| 字段 | 类型 | 用途 |
|------|------|------|
| `f->wpos`, `f->wbase`, `f->wend` | `unsigned char *` | 写缓冲区指针（写入操作时有效） |
| `f->rpos`, `f->rend` | `unsigned char *` | 读缓冲区指针（读取操作时有效） |
| `f->flags` | `unsigned` | 文件流状态标志（含 `F_EOF`） |
| `f->write` | `size_t (*)(FILE *, const unsigned char *, size_t)` | 底层写入函数指针 |
| `f->seek` | `off_t (*)(FILE *, off_t, int)` | 底层定位函数指针（默认 `__stdio_seek`） |

---

## 函数规约

### 1. \_\_fseeko_unlocked

```c
int __fseeko_unlocked(FILE *f, off_t off, int whence);
```

[Visibility]: Internal (hidden) — musl 内部实现，不对外暴露。由不加锁上下文调用执行实际定位操作。

#### Intent

不加锁的文件流定位引擎。处理缓冲区同步（刷新写缓冲、丢弃读缓冲），然后调用底层 `f->seek` 函数指针执行定位。正确处理 `SEEK_CUR` 时缓冲区中未读数据的偏移补偿。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`，已由 `fdopen`/`fopen` 初始化所有函数指针
- `off`: 相对偏移量
- `whence`: `SEEK_SET`、`SEEK_CUR` 或 `SEEK_END`
- 调用方已持有 `f` 的锁（或不需锁的场景，如 `rewind` 中 `FLOCK` 后调用）

#### 后置条件

**Case 1: 成功**
- `whence` 合法
- 写缓冲区（若 `f->wpos != f->wbase`）已通过 `f->write(f, 0, 0)` 刷写；若刷写失败函数提前返回 `-1`
- 内部写缓冲区指针清零：`f->wpos = f->wbase = f->wend = 0`
- 底层 `f->seek(f, off, whence)` 调用成功
- 内部读缓冲区指针清零：`f->rpos = f->rend = 0`
- `f->flags` 中 `F_EOF` 标志被清除
- 返回 `0`

**Case 2: 失败 — whence 非法**
- `errno` 设置为 `EINVAL`
- 返回 `-1`

**Case 3: 失败 — 写缓冲刷写失败**
- `f->write` 返回后 `f->wpos == 0`（表明写入失败）
- 返回 `-1`

**Case 4: 失败 — 底层 seek 失败**
- `f->seek` 返回 `< 0`
- 返回 `-1`
- errno 由底层 seek 设置

#### 系统算法

```
__fseeko_unlocked(f, off, whence):
  1. 验证 whence ∈ {SEEK_CUR, SEEK_SET, SEEK_END}
     - 不合法: errno = EINVAL, return -1

  2. SEEK_CUR 偏移补偿（缓冲区中已读取但未消耗的数据）
     if whence == SEEK_CUR and f->rend:
       off -= f->rend - f->rpos

  3. 刷写写缓冲区
     if f->wpos != f->wbase:
       f->write(f, 0, 0)         // 触发缓冲区刷写
       if !f->wpos:              // write 失败
         return -1

  4. 离开写模式
     f->wpos = f->wbase = f->wend = 0

  5. 执行底层定位
     if f->seek(f, off, whence) < 0:
       return -1

  6. 丢弃读缓冲区并清除 EOF 标志
     f->rpos = f->rend = 0
     f->flags &= ~F_EOF
     return 0
```

#### 不变量

- 函数结束时，若成功则 `f->rpos == f->rend == 0`（读缓冲区被丢弃）且 `f->wpos == f->wbase == f->wend == 0`（写缓冲区清零）
- `F_EOF` 标志在成功定位后被清除（定位使 EOF 条件失效）

#### 依赖

- `f->write` — 底层写入函数指针（由 `__fdopen`/`__fopen_rb_ca` 初始化，默认 `__stdio_write`）
- `f->seek` — 底层定位函数指针（默认 `__stdio_seek`，定义于 `src/stdio/__stdio_seek.c`）
- `<errno.h>` — `EINVAL`

---

### 2. \_\_fseeko

```c
int __fseeko(FILE *f, off_t off, int whence);
```

[Visibility]: Internal (hidden) — 加锁版本的定位函数。由 `fseek`（通过直接调用）和 `fsetpos`（通过 `SEEK_SET`）调用。

#### Intent

获取文件流锁，执行定位操作，释放锁。是 `fseeko` 的主实现（`fseeko` 为其弱别名）。

#### 前置条件

- 同 `__fseeko_unlocked`
- 调用方不持有 `f` 的锁（或持有外层锁则需要 `lockcount` 管理，由 `FLOCK` 内部宏处理）

#### 后置条件

**Case 1: 成功**
- `FLOCK(f)` 获取锁成功
- `__fseeko_unlocked(f, off, whence)` 返回 `0`
- `FUNLOCK(f)` 释放锁
- 返回 `0`

**Case 2: 失败**
- `__fseeko_unlocked` 返回 `-1`
- 锁被释放
- 返回 `-1`

#### 系统算法

```
__fseeko(f, off, whence):
  FLOCK(f)
  result = __fseeko_unlocked(f, off, whence)
  FUNLOCK(f)
  return result
```

#### 依赖

- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（`stdio_impl.h`），内部调用 `__lockfile` / `__unlockfile`
- `__fseeko_unlocked` — 不加锁定位（同文件）

---

### 3. fseek

```c
int fseek(FILE *f, long off, int whence);
```

[Visibility]: User — 标准 C 库函数（ISO C / POSIX），声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

为 `long` 类型偏移量提供标准文件定位接口。内部委托给 `__fseeko`（接受 `off_t` 偏移量）。

#### 前置条件

- 同 `__fseeko`

#### 后置条件

- 与 `__fseeko` 完全相同
- 返回值由 `__fseeko` 直接传递

#### 系统算法

```
fseek(f, off, whence):
  return __fseeko(f, off, whence)
```

#### 依赖

- `__fseeko` — 加锁定位（同文件）

---

### 4. fseeko (weak_alias)

```c
weak_alias(__fseeko, fseeko);
```

[Visibility]: User — POSIX 标准函数，声明于 `<stdio.h>`（需 `_POSIX_C_SOURCE >= 200112L`）。

- **Intention**: 与 `__fseeko` 共享同一实现（接受 `off_t` 偏移量，支持大文件）。前置/后置条件完全等同于 `__fseeko`。
