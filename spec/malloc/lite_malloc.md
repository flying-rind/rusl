# lite_malloc.c 规约

> 轻量级 bump 分配器，用于 musl libc 早期初始化阶段以及作为 `malloc` 的弱符号默认实现。
> 当完整的 malloc 实现（mallocng / oldmalloc）初始化后，会通过 `__libc_malloc_impl` 的强符号
> 覆盖本文件提供的弱符号，从而替换掉简易分配逻辑。

---

## 依赖图

```
malloc (weak, Public)
  └── default_malloc (static)
        └── __libc_malloc_impl (weak alias of __simple_malloc)
              └── __simple_malloc (static)
                    ├── traverses_stack_p (static)
                    │     └── libc.auxv (see libc.h)
                    ├── LOCK / UNLOCK → __lock / __unlock (see lock.h)
                    ├── __syscall(SYS_brk, ...) (see syscall.h)
                    ├── __mmap(...) (see internal sys/mman.h)
                    └── PAGE_SIZE = libc.page_size (see libc.h)

__libc_malloc (Internal, exported)
  └── __libc_malloc_impl (同上)

__bump_lockptr (Internal, exported)
  └── lock (static)
```

---

## 一、内部宏定义

### ALIGN（最小对齐常量）

```c
#define ALIGN 16
```

[Visibility]: Internal — 模块内部宏，POSIX/ISO C 标准未定义

- **值**: 16 字节
- **语义**: bump 分配器的最小对齐粒度。所有分配地址按不大于此值的 2 的幂向上对齐。
- **设计理由**: 16 字节对齐满足 x86_64 等架构上 `long double`、`__int128` 等类型的对齐需求。

---

## 二、内部全局数据

### lock（分配器锁）

```c
static volatile int lock[1];
```

[Visibility]: Internal — `static` 变量，仅在当前编译单元可见

- **类型**: `volatile int[1]`，实际作为单元素整数用作自旋锁标记
- **语义**: 保护 `__simple_malloc` 内部静态变量（`brk`、`cur`、`end`、`mmap_step`）的互斥锁
- **访问规则**: 只通过 `LOCK(lock)` / `UNLOCK(lock)` 宏操作，最终委托给 `__lock` / `__unlock`（see lock.h spec）
- **不变量**: `lock[0]` 为 0 时表示无竞争，非 0 时表示已被持有

### __bump_lockptr（对外暴露的锁指针）

```c
volatile int *const __bump_lockptr = lock;
```

[Visibility]: Internal (不导出给用户) — musl 内部符号，由 `src/internal/fork_impl.h` 声明为 `extern hidden`；被 `src/process/fork.c` 的 fork 处理器引用，用于在 `fork()` 之前锁定 bump 分配器以防止死锁

- **类型**: `volatile int *const`，指向 `lock` 的常量指针
- **语义**: fork 安全机制所需的锁指针，fork 前由 `__malloc_atfork` 加锁，`__post_Fork` 解锁
- **不变量**: 始终指向 `lock`

---

## 三、内部辅助函数

### traverses_stack_p —— 栈区间冲突检测

```c
static int traverses_stack_p(uintptr_t old, uintptr_t new);
```

[Visibility]: Internal — `static` 函数，仅在当前编译单元可见

#### 意图
检测 `brk` 扩展区间 `[old, new)` 是否会与主线程栈或当前线程栈区域发生交叉，作为对有缺陷的 `brk` 实现（可能跨越栈区域的）的白名单防御。

#### 前置条件
- `old` 和 `new` 为有效的虚拟地址（`uintptr_t`），表示提议的堆扩展区间下界和上界
- `new >= old`（调用者保证 `req > 0` 且 `brk + req` 不溢出）
- `libc.auxv` 已初始化（指向内核传递的辅助向量）

#### 后置条件
- **Case 1 — 返回值 1**: 区间 `[old, new)` 与以下区域之一存在交集：
  - 区间 `[max(0, auxv - 8MB), auxv)`（推测为主线程栈区域）
  - 区间 `[max(0, &b - 8MB), &b)`（当前线程栈区域，`b` 为调用时刻的栈帧地址）
- **Case 2 — 返回值 0**: 未检测到与上述区域的冲突，`brk` 扩展可以安全执行

#### 算法
采用 8MB（`8<<20`）作为栈区域深度估计。以 `libc.auxv` 和当前栈指针 `&b` 分别作为两个栈区域的顶部，向下取 8MB 区间，用区间重叠判定 `new > a && old < b` 检测冲突。

#### 局限性
- 8MB 是启发式常量，不等于实际的 `RLIMIT_STACK`；保守起见，若栈在 8MB 以下则可能漏报，但不会导致误杀
- 依赖 `libc.auxv` 恰好位于主线程栈"上方"的假设（Linux 内核将 auxv 放置在高地址区）

#### 依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `libc.auxv` | `src/internal/libc.h` | `__libc` 结构体成员，指向内核辅助向量 |
| `uintptr_t` | `<stdint.h>` | 无符号整数类型，可容纳指针值 |

---

## 四、核心实现函数

### __simple_malloc —— bump 分配器核心实现

```c
static void *__simple_malloc(size_t n);
```

[Visibility]: Internal — `static` 函数；通过 `weak_alias(__simple_malloc, __libc_malloc_impl)` 以弱符号形式间接暴露给 libc 内部其他编译单元。若其他 malloc 实现提供了 `__libc_malloc_impl` 的强符号定义，则弱符号被覆盖，本函数不再被调用。

#### 意图
实现一个极简的 bump 分配器，优先通过 `brk` 系统调用扩展数据段（heap），在 `brk` 不可用或可能导致栈冲突时回退到 `mmap`。对于大块分配（浪费超过 1/8 时），采用几何增长的独立 `mmap` 区域以减少碎片。

#### 前置条件
- `n` 为请求分配的大小（字节）
- `lock` 处于可获取状态（无死锁风险）
- 系统调用 `SYS_brk` 和 `__mmap` 可用（内核已初始化）

#### 后置条件

**Case 1 — 参数非法 (`n > SIZE_MAX/2`)**:
- 返回值: `NULL`（`0`）
- `errno` 设置为 `ENOMEM`
- 堆状态不变

**Case 2 — 分配成功（brk 路径）**:
- 返回值: 指向新分配内存的指针，地址按 `min(2^k, 16)` 对齐，其中 `2^k` 为不小于 `n` 的最小 2 的幂
- 分配的内存位于数据段（heap），紧邻之前已分配的区域
- `cur` 自增 `n`，`end` 可能因 `brk` 扩展而增加
- 锁已释放

**Case 3 — 分配成功（mmap 直接返回，小请求）**:
- `mmap` 成功且 `new_area == 0`
- 返回值: `mmap` 返回的内存地址，页对齐
- 锁已释放
- 不修改 `cur`、`end`、`brk`

**Case 4 — 分配成功（mmap 新区域）**:
- `mmap` 成功且 `new_area == 1`
- `cur` 设为 mmap 返回地址，`end` 设为 `cur + req`
- `mmap_step` 可能递增（最大到 12）
- 返回值: 从新 mmap 区域分配的指针，按 bump 逻辑对齐
- 锁已释放

**Case 5 — 分配失败（mmap 失败）**:
- 返回值: `NULL`（`0`）
- `errno` 由 `__mmap` 设置
- 锁已释放
- 堆状态不变（`cur`、`end`、`brk` 未修改）

#### 算法（System Algorithm）

```
1. 参数校验
   if n > SIZE_MAX/2:
       errno = ENOMEM; return NULL
   if n == 0: n = 1

2. 对齐计算（2 的幂指数增长，上限 ALIGN=16）
   align = 1
   while align < n && align < ALIGN:
       align += align

3. 加锁 LOCK(lock)

4. 地址对齐
   cur = cur + ((-cur) & (align - 1))  // 向上对齐到 align 边界

5. 空间不足时的扩展逻辑
   if n > end - cur:
       req = page_align(n - (end - cur))

       // 首次调用：获取初始 brk
       if cur == 0:
           brk = page_align(sys_brk(0))
           cur = end = brk

       // 尝试 brk 扩展（优先路径）
       if brk == end && req < SIZE_MAX - brk
          && !traverses_stack_p(brk, brk + req)
          && sys_brk(brk + req) == brk + req:
           brk = end += req

       // 回退到 mmap（brk 失败或不可用）
       else:
           req = page_align(n)  // 重新计算：只需对齐本次请求
           new_area = 0

           // 启发式：浪费超过 1/8 时创建新区域
           if req - n > req / 8:
               min = PAGE_SIZE << (mmap_step / 2)
               if min - n > end - cur:  // 用新区域剩余更少
                   req = max(req, min)
                   if mmap_step < 12: mmap_step++
                   new_area = 1

           mem = mmap(NULL, req, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0)
           if mem == MAP_FAILED || !new_area:
               UNLOCK(lock)
               return (mem == MAP_FAILED) ? NULL : mem

           cur = (uintptr_t)mem
           end = cur + req

6. 从当前区域分配（bump）
   p = (void *)cur
   cur += n

7. 解锁并返回
   UNLOCK(lock)
   return p
```

**关键参数**:
- `mmap_step` 初始为 0，每次创建新 mmap 区域最多递增到 12
- `mmap_step` 的最大值 12 对应 `PAGE_SIZE << 6 = 64 * PAGE_SIZE` 的最大几何增长
- 浪费比例阈值 `req - n > req / 8`（即浪费 > 12.5%）触发独立 mmap 区域策略
- 新区域最小尺寸: `PAGE_SIZE << (mmap_step / 2)`（几何增长因子 sqrt(2)）

#### 不变量
- 函数通过 `LOCK`/`UNLOCK` 保证对 `brk`、`cur`、`end`、`mmap_step` 的互斥访问
- `cur` 始终满足 `cur >= brk` 且 `cur <= end`
- 分配的指针始终满足最小对齐要求
- 函数不持有锁退出（包括所有失败路径）

#### 依赖

| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `LOCK` / `UNLOCK` | `src/internal/lock.h` | 自旋锁/线程锁宏，委托给 `__lock`/`__unlock` |
| `__syscall(SYS_brk, ...)` | `src/internal/syscall.h` | 直接发起 `brk` 系统调用 |
| `__mmap` | `src/include/sys/mman.h` | musl 内部 mmap 封装 |
| `PAGE_SIZE` | `src/internal/libc.h` (宏展开为 `libc.page_size`) | 运行时页面大小 |
| `traverses_stack_p` | 本文件 static 函数 | 栈冲突检测 |
| `SIZE_MAX` | `<stdint.h>` / `<limits.h>` | `size_t` 的最大值 |
| `ENOMEM` | `<errno.h>` | 内存不足错误码 |
| `MAP_FAILED` / `PROT_READ` / `PROT_WRITE` / `MAP_PRIVATE` / `MAP_ANONYMOUS` | `<sys/mman.h>` | mmap 相关常量 |

---

## 五、内部导出函数

### __libc_malloc_impl（弱符号分发）

```c
weak_alias(__simple_malloc, __libc_malloc_impl);
```

[Visibility]: Internal (不导出给用户) — 弱符号，由 `weak_alias` 宏生成，具有 `__weak__` 属性。musl 内部各模块通过此符号间接调用 malloc 实现。完整的 malloc 实现（mallocng/oldmalloc）提供同名的强符号定义，覆盖此弱符号。

**语义**: 编译/链接层间接跳板。若未链接强符号定义，则跳转到 `__simple_malloc`；否则跳转到完整 malloc 实现。

---

### __libc_malloc —— libc 内部 malloc 入口

```c
void *__libc_malloc(size_t n);
```

[Visibility]: Internal (不导出给用户) — musl 内部 API，非 `static` 且无 `hidden` 修饰（区别于 `__libc_malloc_impl`），被 libc 内部函数（如 `calloc`、`strdup`、`printf` 系列等）在完整 malloc 初始化前调用。

#### 意图
为 libc 内部使用者提供统一的 `malloc` 调用入口，间接委托给 `__libc_malloc_impl`。间接调用的设计使得运行时替换 malloc 实现成为可能：当完整 malloc 初始化完毕、替换 `__libc_malloc_impl` 的强符号后，本函数无需修改即可自动路由到新实现。

#### 前置条件
- `n` 为请求分配的大小
- `__libc_malloc_impl` 符号已解析（弱符号至少由 `__simple_malloc` 提供）

#### 后置条件
- 返回值与 `__libc_malloc_impl(n)` 一致
- 所有前置/后置条件继承自 `__libc_malloc_impl` 的当前绑定实现

#### 依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `__libc_malloc_impl` | 本文件弱别名 / 完整 malloc 强符号 | 实际的分配函数 |

---

## 六、对外导出函数

### default_malloc / malloc —— POSIX 标准 malloc

```c
static void *default_malloc(size_t n);
weak_alias(default_malloc, malloc);
```

`malloc` 函数签名：
```c
void *malloc(size_t size);
```

[Visibility]: **Public** — POSIX.1-2001 标准函数，声明于 `<stdlib.h>`。通过 `weak_alias(default_malloc, malloc)` 以弱符号形式导出，通常被完整 malloc 实现（mallocng 或 oldmalloc）的强符号覆盖。若因链接配置导致完整 malloc 未被包含，则回退到本 bump 分配器提供基本可用性。

#### 意图
提供符合 POSIX 标准的动态内存分配接口。在正常 musl 构建中，此弱符号被完整 malloc 实现的强符号 `malloc` 覆盖；本文件版本仅作为链接时回退（fallback）或早期启动阶段的临时实现。

#### 前置条件
- `size` 为请求分配的字节数
- 若 `size == 0`，行为由实现定义（本实现返回一个有效指针，等同于 `size = 1`）

#### 后置条件
- **Case 1 — 分配成功**:
  - 返回值: 指向至少 `size` 字节已分配内存的指针，适当对齐，内容未初始化
  - 返回的指针可安全传递给 `free()`、`realloc()` 等函数（前提是 `free` 实现能处理 bump 分配器产生的指针——完整 malloc 通常接管后使用自己的元数据）
- **Case 2 — 分配失败**:
  - 返回值: `NULL`
  - `errno` 设置为 `ENOMEM`
- **注意**: 本实现（bump 分配器）**不支持 `free()`**。通过弱符号覆盖机制，正式构建中 `malloc` 通常被完整分配器替换，不存在此问题。若回退到此实现且调用者尝试 `free()`，将导致未定义行为。

#### 算法
直接委托: `return __libc_malloc_impl(n);`

#### 不变量
- 函数无内部状态，不持有锁跨越调用边界
- 线程安全由 `__libc_malloc_impl` 保证

#### 依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `__libc_malloc_impl` | 本文件弱别名 / 完整 malloc 强符号 | 实际的分配逻辑 |

---

## 跨模块依赖汇总

| 外部依赖 | 来源文件 | 用途 |
|----------|----------|------|
| `__lock` / `__unlock` | `src/thread/__lock.c` | bump 分配器互斥锁 |
| `__syscall` | `src/internal/syscall.h` + 架构相关 `syscall_arch.h` | 发起 `brk` 系统调用 |
| `__mmap` | `src/mman/mmap.c` | 匿名内存映射 |
| `libc.page_size` | `src/internal/libc.h` (`struct __libc`) | 运行时页面大小 |
| `libc.auxv` | 同上 | 栈区间检测参考点 |
| `__bump_lockptr` | 本文件定义，`src/process/fork.c` 引用 | fork 安全锁 |

---

## 设计备注

1. **弱符号覆盖机制**: musl 采用静态链接期弱/强符号替换策略。`lite_malloc.c` 同时提供 `__libc_malloc_impl`（弱）和 `malloc`（弱）两个弱符号。当链接 `mallocng` 或 `oldmalloc` 时，这些目标文件中的强符号会覆盖弱符号。这意味着在正式构建中，本文件的 `__simple_malloc` 代码可能在死代码消除（DCE）阶段被完全移除。

2. **bump 分配器的语义限制**: `__simple_malloc` 是"纯增量"分配器，分配的内存**不可被释放**。它仅设计用于：
   - libc 早期初始化阶段（在完整 malloc 接管之前分配少量持久对象）
   - 极端链接配置下作为最后的 fallback

3. **brk 优先策略**: 优先通过 `brk` 扩展堆，因为 `brk` 在数据段范围内连续，缓存局部性优于 `mmap` 的随机地址分配。仅当 `brk` 可能穿过栈区域或 `brk` 系统调用失败时回退到 `mmap`。

4. **mmap 区域几何增长**: `mmap_step` 的几何增长策略（`min = PAGE_SIZE << (mmap_step/2)`）使连续的大块请求倾向于复用同一 mmap 区域，减少系统调用次数和 VMA 碎片。