# ofl.c 规约

> musl libc 全局打开文件链表（open file list）管理实现。提供加锁和解锁访问全局 FILE 链表的接口。该链表用于 `__stdio_exit` 在程序退出时遍历所有打开的流以刷新缓冲区。

---

## 依赖图

```
ofl.c
  ├── __ofl_lock (Internal) ──> LOCK (of ofl_lock)
  ├── __ofl_unlock (Internal) ──> UNLOCK (of ofl_lock)
  ├── __stdio_ofl_lockptr (Internal) ──> 指向 ofl_lock 的指针
  ├── ofl_head (static, 全局文件链表头)
  └── ofl_lock (static, 链表锁)
```

---

## 内部静态变量

### 1. ofl_head

```c
static FILE *ofl_head;
```

[Visibility]: Internal — `static` 变量，仅本文件内可见

全局打开文件链表的头指针。链表通过每个 FILE 对象的 `prev`/`next` 字段构成双向链表。新流通过 `__ofl_add` 添加到链表头部。

### 2. ofl_lock

```c
static volatile int ofl_lock[1];
```

[Visibility]: Internal — `static` 变量，仅本文件内可见

保护 `ofl_head` 的自旋锁。`volatile` 修饰确保每次访问从内存读取，防止编译器优化。使用 `LOCK`/`UNLOCK` 宏（底层调用 `__lock` / `__unlock`）进行操作。

---

## 全局符号规约

### 3. \_\_stdio_ofl_lockptr

```c
volatile int *const __stdio_ofl_lockptr = ofl_lock;
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用

指向 `ofl_lock` 的常量指针。主要用于 `fork` 子进程中重置锁状态：`fork_impl.h` 中的 `__reinit_locks` 通过此指针将 `ofl_lock` 清零。

---

## 函数规约

### 4. \_\_ofl_lock

```c
FILE **__ofl_lock(void);
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用

#### Intent

获取全局打开文件链表锁，并返回链表头指针的地址。调用方可通过返回的 `FILE **` 指针遍历或修改链表。调用方必须随后调用 `__ofl_unlock()` 释放锁。

该函数是链表的唯一入口点，任何需要遍历或修改全局 FILE 链表的代码必须经过此函数。

#### 前置条件

- `ofl_lock` 未被当前线程持有（禁止递归锁）

#### 后置条件

- `ofl_lock` 被当前线程持有（通过 `LOCK(ofl_lock)`）
- 返回 `&ofl_head` 的指针，调用方可安全读取/修改链表
- 调用方必须在完成操作后调用 `__ofl_unlock()`

#### 系统算法

```
__ofl_lock():
  1. LOCK(ofl_lock) — 自旋锁获取
  2. return &ofl_head
```

---

### 5. \_\_ofl_unlock

```c
void __ofl_unlock(void);
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用

#### Intent

释放全局打开文件链表锁。与 `__ofl_lock()` 配对使用。

#### 前置条件

- `ofl_lock` 被当前线程持有

#### 后置条件

- `ofl_lock` 被释放（通过 `UNLOCK(ofl_lock)`）

#### 系统算法

```
__ofl_unlock():
  1. UNLOCK(ofl_lock) — 自旋锁释放
```

---

## 不变量

- 任何对 `ofl_head` 的读取或修改必须在持有 `ofl_lock` 的前提下进行
- `__ofl_lock()` 和 `__ofl_unlock()` 必须成对调用
- `__stdio_ofl_lockptr` 始终指向 `ofl_lock`，用于 `fork` 后的锁重置

## 依赖

- `LOCK` / `UNLOCK` 宏 — 自旋锁操作（定义于 `src/internal/lock.h`，底层调用 `__lock` / `__unlock`）
- `struct _IO_FILE` — FILE 结构体（定义于 `src/internal/stdio_impl.h`）
