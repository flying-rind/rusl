# fflush.c 规约

> musl libc 标准库流刷新函数实现。将流缓冲区的未写入数据写出到实际文件/设备，同步读取位置。

---

## 依赖图

```
fflush
  ├─> __stdout_used / __stderr_used (weak refs, see stdout.c / stderr.c)
  ├─> __ofl_lock() / __ofl_unlock() (see ofl.c spec)
  ├─> FLOCK(f) / FUNLOCK(f)  (宏, see stdio_impl.h)
  ├─> f->write(f, 0, 0)      (通过 FILE 对象函数指针调用)
  ├─> f->seek(f, ...)        (通过 FILE 对象函数指针调用)
  └─> fflush_unlocked (weak_alias)
```

---

## 内部辅助定义

### dummy / __stdout_used / __stderr_used

```c
static FILE *volatile dummy = 0;
weak_alias(dummy, __stdout_used);
weak_alias(dummy, __stderr_used);
```

[Visibility]: Internal (不导出) — 弱符号占位符。当链接了 `stdout.c` 或 `stderr.c` 时，这些符号被替换为指向实际 `stdout`/`stderr` 的指针；否则为 `NULL`。

- **Intention**: 控制对所有打开流的全刷新操作的范围。若 `stdout`/`stderr` 被链接，它们将被包含在全刷新遍历中。

---

## 函数规约

### 1. fflush

```c
int fflush(FILE *f);
```

[Visibility]: User — 声明于 `<stdio.h>`，用户程序可直接调用

### 2. fflush_unlocked (weak_alias)

```c
weak_alias(fflush, fflush_unlocked);
```

[Visibility]: User — 声明于 `<stdio.h>`，POSIX 标准接口。与 `fflush` 行为完全相同，但不执行内部锁定（调用者需自行保证线程安全）。

---

#### Intent

刷新 `FILE` 流的缓冲区：若写入模式下有未写出数据，将其写出；若读取模式下有预读数据，将其位置同步回底层文件。若参数 `f` 为 `NULL`，则刷新所有当前打开的流。

#### 前置条件

- 若 `f != NULL`:
  - `f` 必须是一个有效的已打开 `FILE*` 指针
  - 调用者（对于 `fflush`，非 `fflush_unlocked`）无需持有锁，函数内部会锁住 `f`
- 若 `f == NULL`:
  - 触发全局刷新所有打开文件流（包括 stdout/stderr 若已链接）

#### 后置条件

- **Case 1: f != NULL 且成功**
  - 若 `f->wpos != f->wbase`（有未写出的缓冲数据）：
    - 调用 `f->write(f, 0, 0)` 将缓冲区数据写出
    - 若写出后 `f->wpos == 0`（write 返回错误），返回 `EOF`
  - 若 `f->rpos != f->rend`（有未读的预读数据）：
    - 调用 `f->seek(f, f->rpos - f->rend, SEEK_CUR)` 将文件偏移量回退到实际已读位置
  - 清除所有读写模式指针：`f->wpos = f->wbase = f->wend = 0; f->rpos = f->rend = 0`
  - 返回 `0`

- **Case 2: f == NULL**
  - 若 `__stdout_used != NULL`，刷 `fflush(__stdout_used)`
  - 若 `__stderr_used != NULL`，刷 `fflush(__stderr_used)`
  - 遍历所有打开文件链表（`__ofl_lock` 获取链表，每个文件 `FLOCK`/`FUNLOCK`），若 `wpos != wbase` 则刷新
  - 返回所有 `fflush` 调用的合并结果（按位 OR）

- **Case 3: f != NULL 且写出失败**
  - `f->write` 将 `f->wpos` 设为 `0` 以标记写入错误
  - 返回 `EOF`

#### 不变量

- 刷新操作后，FILE 对象的读/写缓冲区被重置为空闲状态（`rpos = rend = 0`, `wpos = wbase = wend = 0`）
- 全局刷新时对打开文件链表持有锁，保证快照一致性

#### 系统算法

```
fflush(f):
  if (f == NULL):                              // 刷新所有流
    r = 0
    if (__stdout_used) r |= fflush(__stdout_used)
    if (__stderr_used) r |= fflush(__stderr_used)
    for (f = *__ofl_lock(); f; f = f->next):   // 遍历所有打开文件
      FLOCK(f)
      if (f->wpos != f->wbase) r |= fflush(f)  // 尾递归: 此 inner fflush 不再为 f==NULL
      FUNLOCK(f)
    __ofl_unlock()
    return r

  // 刷新单个流
  FLOCK(f)
  if (f->wpos != f->wbase):                    // 写入模式: 有未写出数据
    f->write(f, 0, 0)                          // 调用底层写出
    if (!f->wpos):                             // write 将 wpos 置 0 表示错误
      FUNLOCK(f); return EOF
  if (f->rpos != f->rend):                     // 读取模式: 有未消费的预读数据
    f->seek(f, f->rpos - f->rend, SEEK_CUR)    // 回退文件位置

  f->wpos = f->wbase = f->wend = 0             // 重置写入缓冲区
  f->rpos = f->rend = 0                        // 重置读取缓冲区
  FUNLOCK(f)
  return 0
```

#### 依赖

- `__stdout_used` / `__stderr_used` — 弱符号，指向 stdout/stderr 的指针（定义于 `src/stdio/stdout.c` / `stderr.c`）
- `__ofl_lock()` / `__ofl_unlock()` — 获取/释放全局打开文件链表锁（定义于 `src/stdio/ofl.c`）
- `FLOCK(f)` / `FUNLOCK(f)` — 宏，锁/解锁 FILE 对象（定义于 `src/internal/stdio_impl.h`）
- `f->write(f, 0, 0)` — 通过 `FILE` 对象的函数指针调用底层写出操作
- `f->seek(f, offset, whence)` — 通过 `FILE` 对象的函数指针调用底层 seek 操作
