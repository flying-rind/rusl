# putc.h 规约

> musl libc 内部 stdio 字符写入辅助模块。定义 `do_putc` 和 `locking_putc`，为 `fputc`、`putc`、`putchar` 等公开 API 提供统一的加锁字符写入逻辑。

---

## 依赖图

```
do_putc (inline)
  ├── putc_unlocked (宏, 定义于 stdio_impl.h)
  │     └── __overflow (see __overflow.c spec)
  ├── locking_putc (static)
  │     ├── a_cas (原子CAS, 来自内部 atomic.h)
  │     ├── __lockfile (see __lockfile.c spec)
  │     ├── putc_unlocked (同上)
  │     ├── a_swap (原子交换, 来自内部 atomic.h)
  │     └── __wake (futex唤醒, 来自内部)
  └── __pthread_self (see pthread_self.c spec)
```

---

## 函数规约

### 1. locking_putc

```c
// 定义于 putc.h，static 函数，仅本翻译单元可见
#ifdef __GNUC__
__attribute__((__noinline__))
#endif
static int locking_putc(int c, FILE *f);
```

[Visibility]: Internal — `static` 函数，不对外导出

#### Intent

为当前线程尚未持有锁的 FILE 提供完整的"加锁-写入-解锁"原子操作。使用原子 CAS 获取锁，若锁已被占用则阻塞等待，写入完成后使用原子 swap 释放锁，并在必要时唤醒等待者。

#### 前置条件

- `c`: 要写入的字符（以 `int` 传递，内部转为 `unsigned char`）
- `f`: 非空 FILE 指针，指向有效的 FILE 对象
- 当前线程**不持有** `f->lock`
- `f->lock >= 0`

#### 后置条件

- **Case 1 成功写入字符**
  - 返回写入的字符（以 `unsigned char` 范围的正值）
  - 锁已释放

- **Case 2 写入失败**
  - 返回 `EOF`（通常为 -1）
  - 锁已释放

#### 系统算法

```
locking_putc(c, f):
  1. 使用 a_cas(&f->lock, 0, MAYBE_WAITERS-1) 尝试获取锁
     - 若 lock 不为 0，调用 __lockfile(f) 阻塞等待
  2. 调用 putc_unlocked(c, f) 执行实际写入
  3. 使用 a_swap(&f->lock, 0) 释放锁
     - 若旧值含有 MAYBE_WAITERS 标志，调用 __wake(&f->lock, 1, 1) 唤醒一个等待者
  4. 返回写入结果
```

#### 依赖

- `a_cas` — 原子 compare-and-swap
- `a_swap` — 原子交换
- `__lockfile(FILE *)` — 阻塞获取 FILE 锁
- `__wake` — futex 唤醒
- `putc_unlocked` — 无锁字符写入宏（定义于 `stdio_impl.h`）

---

### 2. do_putc

```c
static inline int do_putc(int c, FILE *f);
```

[Visibility]: Internal — `static inline` 函数，每个包含 `putc.h` 的翻译单元内联展开

#### Intent

智能锁检查的字符写入入口。若 FILE 已处于免锁模式或当前线程已持有锁，则直接调用 `putc_unlocked` 避免重复加锁；否则走完整的 `locking_putc` 加锁路径。此设计支持三场景：
1. 调用者在外部已获取锁时（如 `fwrite` 内部循环），避免锁递归
2. FILE 为免锁 FILE（`lock < 0`）
3. 常规路径，需要完整加锁

#### 前置条件

- `c`: 要写入的字符
- `f`: 非空 FILE 指针

#### 后置条件

- 返回值同 `putc_unlocked`：成功返回写入的字符，失败返回 `EOF`
- 不改变 `f->lock` 的所有权状态

#### 系统算法

```
do_putc(c, f):
  l = f->lock
  if (l < 0)                                 // 免锁 FILE
     or (l != 0 and (l & ~MAYBE_WAITERS) == __pthread_self()->tid)  // 当前线程持有锁
  then
    return putc_unlocked(c, f)               // 无需加锁，直接写入
  else
    return locking_putc(c, f)                // 需要加锁
```

#### 依赖

- `putc_unlocked` — 无锁字符写入宏
- `locking_putc(int, FILE *)` — 同文件 static 函数
- `__pthread_self()` — 获取当前线程控制块指针
- `MAYBE_WAITERS` — 锁标志常量
