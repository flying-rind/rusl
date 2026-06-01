# __reset_tls.c 规约

## 依赖图

```
__reset_tls
  ├── __pthread_self()          [pthread_impl.h, 宏展开为 __get_tp() + 偏移]
  ├── struct tls_module         [libc.h]
  ├── libc (即 __libc 全局变量)   [libc.h]
  ├── DTP_OFFSET                [pthread_impl.h, 编译时常量]
  ├── memcpy()                  [外部 libc, <string.h>]
  └── memset()                  [外部 libc, <string.h>]
```

> **说明**: `__reset_tls.c` 自身未定义任何 `static` 函数或内部结构体。所有依赖均来自内部头文件（`pthread_impl.h`、`libc.h`）或外部 libc。调用点：`src/time/timer_create.c:39`，在 `cleanup_fromsig()` 中。

---

## 内部类型引用

以下类型定义于其他模块，在此处仅做简要说明以支撑规约理解。

### `struct tls_module` (libc.h)

[Visibility]: Internal — musl 内部 TLS 模块描述符，不在 POSIX 标准中定义

```c
struct tls_module {
    struct tls_module *next;  // 单向链表，指向下一个已加载的 TLS 模块
    void *image;              // 指向 TLS 模板块初始数据的原型镜像
    size_t len;               // 已初始化数据段大小（.tdata）
    size_t size;              // TLS 块总大小（.tdata + .tbss）
    size_t align;             // 对齐要求
    size_t offset;            // 模块在 DTV 中的偏移编排信息
};
```

**不变量**: `len <= size` 始终成立（已初始化数据不超过总大小）。`image` 指向的初始数据在程序生命周期内不可变。

### `struct pthread` — DTV 相关字段 (pthread_impl.h)

[Visibility]: Internal — musl 内部线程控制块

```c
struct pthread {
    // ...
    uintptr_t *dtv;   // 指向 Dynamic Thread Vector 数组
    // ...
};
```

`dtv` 是一个数组，约定如下：
- `dtv[0]` 存储当前线程绑定的 TLS 模块数 `n`。
- 对于 `i ∈ [1, n]`，`dtv[i]` 存储模块 i 的 TLS 块"已偏置指针"；通过 `dtv[i] - DTP_OFFSET` 可得该模块 TLS 块的真实起始地址。

### `libc` 全局变量 (libc.h)

[Visibility]: Internal — musl 内部全局运行时状态

```c
extern hidden struct __libc __libc;
#define libc __libc
```

`__libc` 的 `tls_head` 字段指向已加载 TLS 模块的单向链表头，遍历顺序与 DTV 索引 i 一致（`tls_head` 对应 i=1，`tls_head->next` 对应 i=2，以此类推）。

### `DTP_OFFSET` 宏 (pthread_impl.h)

[Visibility]: Internal — 架构相关的编译时常量

```c
#ifndef DTP_OFFSET
#define DTP_OFFSET 0
#endif
```

DTP_OFFSET 是 dtv 指针与 TLS 块起始地址之间的固定偏移量。在大多数架构上为 0。`dtv[i]` = TLS块起始地址 + DTP_OFFSET。

---

## 函数规约

---

### `__reset_tls`

[Visibility]: Internal (不导出) — musl 内部函数，POSIX/C 标准未定义。声明于 `pthread_impl.h` 且标记为 `hidden`，仅用于 `fork()` 之后或定时器信号处理线程的 TLS 重置场景。

#### 签名

```c
void __reset_tls(void);
```

#### 意图 (Intent) — Level 2

将当前线程的所有 TLS（Thread-Local Storage）变量恢复到程序加载时的初始值。此操作对于 `fork()` 后的子进程以及与进程共享地址空间的信号处理线程是必需的：这些执行上下文继承了父线程的 TLS 内存，但其内容可能已偏离初始状态（如 `errno`、`h_errno`、区域设置等），必须重置以确保语义正确。

实现采用"逐模块复制 + 尾零填充"策略：遍历全局 TLS 模块链表，将每个模块的初始镜像拷贝回当前线程的对应 TLS 内存区域，已初始化部分之外的区域清零（对应 `.tbss` 语义）。

#### 前置条件 (Pre-condition)

1. 调用线程必须已通过 TLS 初始化流程，即 `self->dtv` 已分配且 `self->dtv[0]` 已正确设置为已加载 TLS 模块的数量。
2. 全局链表 `libc.tls_head` 已构建完毕，且其中的模块顺序与 DTV 索引 1..n 一一对应。
3. 对于所有 `i ∈ [1, n]`：
   - `(char *)(self->dtv[i] - DTP_OFFSET)` 指向的 TLS 块大小至少为 `p->size` 字节（其中 `p` 为对应模块），且该内存区域可读写。
   - `p->image` 指向至少 `p->len` 字节的有效数据。
4. **调用上下文**: 当前应在单线程环境中执行（或调用者已保证无并发 TLS 访问），以避免 `memcpy`/`memset` 写入 TLS 时与读取产生数据竞争。

#### 后置条件 (Post-condition)

- 若 `n == 0`（当前线程无 TLS 模块）：函数为空操作，直接返回。
- 若 `n > 0`：对于所有 `i ∈ [1, n]`，设 `p = libc.tls_head` 链表中第 `i` 个模块：
  - **Case 1 (成功，总是)**:
    - 地址 `mem = (char *)(self->dtv[i] - DTP_OFFSET)` 处的前 `p->len` 字节等于 `p->image` 的对应字节（即 TLS 已初始化数据恢复到初始值）。
    - 地址 `mem + p->len` 至 `mem + p->size - 1` 的全部字节被置零（即 `.tbss` 段恢复为零初始状态）。
    - 所有 TLS 变量的值等同于程序刚加载时的初始值。
  - **Case 2 (失败)**: 无。此函数不返回错误码，且始终成功（`memcpy`/`memset` 在有效内存范围内不会失败）。

#### 不变量 (Invariants)

无跨此函数维持的不变量。此函数是一次性重置操作，每次调用独立。

#### 系统算法 (System Algorithm) — Level 3

该函数对 musl 的 TLS 运行时正确性至关重要，具体算法如下：

```
Input:  当前线程的 struct pthread *self（通过 __pthread_self() 获取）
        libc.tls_head 全局 TLS 模块链表

Algorithm:
1.  self := __pthread_self()          // 获取当前线程控制块指针
2.  n   := self->dtv[0]               // DTV[0] 存储模块数量
3.  if n == 0: return                 // 无 TLS 模块，直接返回
4.  p := libc.tls_head                // p 指向第一个 TLS 模块
5.  for i := 1 to n:
6.      mem := (char *)(self->dtv[i] - DTP_OFFSET)   // 恢复 TLS 块真实起始地址
7.      memcpy(mem, p->image, p->len)                // 复制初始数据
8.      memset(mem + p->len, 0, p->size - p->len)    // 清零 .tbss 区域
9.      p := p->next                                  // 前进到下一模块
10. end for
```

时间复杂度: O(各模块 size 之和)。每次调用对所有 TLS 模块做全量复制，而非增量更新。

**性能说明**: 此函数采用"全量复制"而非"增量差分"，因为：
1. 无法可靠追踪哪些 TLS 变量被修改过；
2. `fork()` 和信号处理线程的 TLS 重置属于低频操作；
3. 正确性优先于性能 — 确保所有 TLS 变量恢复初始状态。