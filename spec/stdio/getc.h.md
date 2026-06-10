# getc.h 规约

> musl libc 内部 stdio 字符读取辅助模块。定义 `do_getc` 和 `locking_getc`，为 `fgetc`、`getc`、`getchar` 等公开 API 提供统一的加锁字符读取逻辑。

---

## 依赖图

```
do_getc (inline)
  ├── getc_unlocked (宏, 定义于 stdio_impl.h)
  │     └── __uflow (see __uflow.c spec)
  ├── locking_getc (static)
  │     ├── a_cas (原子CAS, 来自内部 atomic.h)
  │     ├── __lockfile (see __lockfile.c spec)
  │     ├── getc_unlocked (同上)
  │     ├── a_swap (原子交换, 来自内部 atomic.h)
  │     └── __wake (futex唤醒, 来自内部)
  └── __pthread_self (see pthread_self.c spec)
```

---

## 宏定义

### MAYBE_WAITERS

```c
#define MAYBE_WAITERS 0x40000000
```

[Visibility]: Internal

用于 FILE.lock 字段的高位标志，表示可能有其他线程在等待该锁。

---

## 函数规约

### 1. locking_getc

```c
// 定义于 getc.h，static 函数，仅本翻译单元可见
#ifdef __GNUC__
__attribute__((__noinline__))
#endif
static int locking_getc(FILE *f);
```

[Visibility]: Internal — `static` 函数，不对外导出

#### Intent

为当前线程尚未持有锁的 FILE 提供完整的"加锁-读取-解锁"原子操作。使用原子 CAS 获取锁，若锁已被占用则阻塞等待，读取完成后使用原子 swap 释放锁，并在必要时唤醒等待者。

#### 前置条件

- `f`: 非空 FILE 指针，指向有效的 FILE 对象
- 当前线程**不持有** `f->lock`（锁由其他线程持有或空闲）
- `f->lock >= 0`（FILE 对象是带锁的常规 FILE，非免锁 FILE）

#### 后置条件

- **Case 1 成功读取字符**
  - 返回读取到的字符（以 `unsigned char` 范围的正值）
  - 锁已释放

- **Case 2 读取失败（EOF 或错误）**
  - 返回 `EOF`（通常为 -1）
  - 锁已释放

#### 系统算法

```
locking_getc(f):
  1. 使用 a_cas(&f->lock, 0, MAYBE_WAITERS-1) 尝试获取锁
     - 若 lock 不为 0，调用 __lockfile(f) 阻塞等待
  2. 调用 getc_unlocked(f) 执行实际读取
  3. 使用 a_swap(&f->lock, 0) 释放锁
     - 若旧值含有 MAYBE_WAITERS 标志，调用 __wake(&f->lock, 1, 1) 唤醒一个等待者
  4. 返回读取结果
```

#### 依赖

- `a_cas` — 原子 compare-and-swap（内部 atomic 操作）
- `a_swap` — 原子交换（内部 atomic 操作）
- `__lockfile(FILE *)` — 阻塞获取 FILE 锁（定义于 `src/stdio/__lockfile.c`）
- `__wake` — futex 唤醒（内部实现）
- `getc_unlocked` — 无锁字符读取宏（定义于 `stdio_impl.h`）

---

### 2. do_getc

```c
static inline int do_getc(FILE *f);
```

[Visibility]: Internal — `static inline` 函数，每个包含 `getc.h` 的翻译单元内联展开

#### Intent

智能锁检查的字符读取入口。若 FILE 已处于免锁模式或当前线程已持有锁，则直接调用 `getc_unlocked` 避免重复加锁；否则走完整的 `locking_getc` 加锁路径。此设计支持三场景：
1. 调用者在外部已获取锁（如 `fread` / `fgets` 内部循环），避免锁递归
2. FILE 为免锁 FILE（`lock < 0`），如 `fmemopen` 创建的流
3. 常规路径，需要完整加锁

#### 前置条件

- `f`: 非空 FILE 指针

#### 后置条件

- 返回值同 `getc_unlocked`：成功返回字符（正数），失败返回 `EOF`
- 不改变 `f->lock` 的所有权状态

#### 系统算法

```
do_getc(f):
  l = f->lock
  if (l < 0)                              // 免锁 FILE (如 fmemopen)
     or (l != 0 and (l & ~MAYBE_WAITERS) == __pthread_self()->tid)  // 当前线程持有锁
  then
    return getc_unlocked(f)               // 无需加锁，直接读取
  else
    return locking_getc(f)                // 需要加锁
```

#### 依赖

- `getc_unlocked` — 无锁字符读取宏
- `locking_getc(FILE *)` — 同文件 static 函数
- `__pthread_self()` — 获取当前线程控制块指针
- `MAYBE_WAITERS` — 锁标志常量
