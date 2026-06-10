# fclose.c 规约

> musl libc 标准库文件关闭函数实现。刷新所有缓冲数据，关闭文件描述符，释放 `FILE` 对象内存。

---

## 依赖图

```
fclose
  ├─> fflush(f)               (see fflush.c spec — 刷新缓冲区)
  ├─> f->close(f)             (see __fdopen.c spec / fmemopen.c spec — FILE 的关闭回调)
  ├─> __unlist_locked_file(f) (see __lockfile.c spec — 从线程锁定文件列表中移除)
  ├─> __ofl_lock()            (see ofl.c spec — 获取打开文件链表锁)
  ├─> __ofl_unlock()          (see ofl.c spec — 释放打开文件链表锁)
  ├─> free(f->getln_buf)      (来自 <stdlib.h> — 释放 getline 缓冲区)
  ├─> free(f)                 (来自 <stdlib.h> — 释放 FILE 对象)
  ├─> FLOCK(f) / FUNLOCK(f)   (宏, see stdio_impl.h — 锁/解锁 FILE 对象)
  └─> dummy / __unlist_locked_file (weak alias, 见下方)
```

---

## 内部辅助定义

### dummy / __unlist_locked_file (weak_alias)

```c
static void dummy(FILE *f) { }
weak_alias(dummy, __unlist_locked_file);
```

[Visibility]: Internal (不导出) — `__unlist_locked_file` 的默认空实现。当链接了 `flockfile.c` 时会被覆盖为真正实现；否则为空操作。

- **Intention**: 弱符号机制。若用户未使用 `flockfile`/`ftrylockfile`，则无需从线程锁定文件链表中移除 `FILE` 对象。

---

## 函数规约

### 1. fclose

```c
int fclose(FILE *f);
```

[Visibility]: User — 声明于 `<stdio.h>`，用户程序可直接调用

#### Intent

关闭一个文件流。首先刷新缓冲区（`fflush`），然后调用 `FILE` 对象的内部 `close` 回调（关闭底层文件描述符或调用自定义关闭函数），最后从全局打开文件链表中移除该 `FILE` 对象并释放其内存。调用后 `f` 为悬空指针，任何后续访问行为均未定义。

#### 前置条件

- `f` 必须是通过 `fopen`、`freopen`、`fmemopen`、`open_memstream`、`fopencookie` 或 `fdopen` 成功打开的合法 `FILE*` 指针
- `f` 不能已被关闭（否则行为未定义）

#### 后置条件

- **Case 1: 成功关闭并释放（非永久文件）**
  - `fflush(f)` 被调用以刷新所有缓冲数据
  - `f->close(f)` 被调用以执行底层关闭操作（关闭文件描述符或回调）
  - 若 `f->flags` 不含 `F_PERM`（即该文件不是通过 `__stdin_used`/`__stdout_used`/`__stderr_used` 等永久存在的流）：
    - `__unlist_locked_file(f)` 被调用（若链接了锁支持）
    - 从全局打开文件链表（`ofl_head`）中移除 `f`
    - `f->getln_buf` 被 `free` 释放
    - `f` 本身被 `free` 释放
  - 返回 `fflush` 与 `f->close` 的合并结果（按位 OR）
- **Case 2: 永久文件**
  - 若 `f->flags & F_PERM` 为真，仅执行 `fflush` + `f->close`，不释放 `FILE` 对象
  - 返回 `fflush` 与 `f->close` 的合并结果

#### 不变量

- 关闭后 `f` 不再有效，任何对 `f` 的访问均为未定义行为
- 即使在关闭过程中发生错误，`FILE` 对象始终从打开文件链表中移除（除非为永久文件）

#### 系统算法

```
fclose(f):
  1. FLOCK(f)                  // 锁定 FILE 对象
  2. r = fflush(f)             // 刷新缓冲区
  3. r |= f->close(f)          // 调用底层关闭回调
  4. FUNLOCK(f)                // 解锁 FILE 对象
  5. if (f->flags & F_PERM)    // 若是永久流(stdin/stdout/stderr)
        return r               // 不释放 FILE 对象
  6. __unlist_locked_file(f)   // 从线程锁定文件链表移除
  7. head = __ofl_lock()       // 获取全局打开文件链表锁
  8. 从链表中移除 f:
        if (f->prev) f->prev->next = f->next
        if (f->next) f->next->prev = f->prev
        if (*head == f) *head = f->next
  9. __ofl_unlock()            // 释放全局锁
  10. free(f->getln_buf)       // 释放 getline 缓冲区
  11. free(f)                  // 释放 FILE 对象
  12. return r
```

#### 依赖

- `fflush(f)` — 刷新 FILE 流缓冲区（定义于 `src/stdio/fflush.c`）
- `__unlist_locked_file(f)` — 弱符号，默认空实现；链接 `flockfile.c` 时有效（定义于 `src/stdio/flockfile.c`）
- `__ofl_lock()` / `__ofl_unlock()` — 获取/释放全局打开文件链表锁（定义于 `src/stdio/ofl.c`）
- `free()` — 标准库内存释放（来自 `<stdlib.h>`）
- `FLOCK(f)` / `FUNLOCK(f)` — 宏，锁/解锁 FILE 对象（定义于 `src/internal/stdio_impl.h`）

#### 错误处理

| 条件 | 返回值 | 说明 |
|------|--------|------|
| `fflush` 或 `f->close` 成功 | `0` | 正常关闭 |
| `fflush` 失败 | 非零 | `fflush` 错误码（通常为 `EOF`） |
| `f->close` 失败 | 非零 | `f->close` 错误码 |
| 两者都失败 | 非零 | 两者按位 OR 的结果 |
