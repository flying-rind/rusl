# __init_tls.c 规约

> **来源文件**: `musl/src/env/__init_tls.c`
> **复杂度层级**: Level 3 — 高度优化设计（ELF 程序头解析 + TLS 布局计算 + 线程控制块初始化 + 条件编译架空构差异）
> **调用者**: `__libc_start_main()` (`src/env/__libc_start_main.c`)，在进程启动早期调用，此时单线程、无锁竞争

---

## 依赖图

```
__init_tls (weak_alias → static_init_tls)
├── [ELF 程序头解析] 依赖 Phdr / Elf64_Phdr 类型 (elf.h)
│   ├── 读取 auxv[AT_PHDR], auxv[AT_PHNUM], auxv[AT_PHENT]
│   └── 遍历 program headers 寻找 PT_PHDR, PT_DYNAMIC, PT_TLS, PT_GNU_STACK
│
├── [main_tls 布局计算] 依赖 struct tls_module (libc.h)
│   ├── 读取 tls_phdr->p_vaddr, p_filesz, p_memsz, p_align
│   ├── 计算 main_tls.size (向上对齐)
│   ├── 计算 main_tls.offset (TLS_ABOVE_TP / !TLS_ABOVE_TP 两种模式)
│   ├── 更新 libc.tls_align, libc.tls_size, libc.tls_cnt, libc.tls_head
│   └── 依赖 MIN_TLS_ALIGN 宏 (本文件定义)、builtin_tls 静态缓冲区
│
├── [内存分配]
│   ├── 若 libc.tls_size ≤ sizeof(builtin_tls) → 使用 builtin_tls[1] 静态缓冲区
│   └── 否则 → __syscall(SYS_mmap2/ SYS_mmap, ...) 匿名映射
│
├── __copy_tls(unsigned char *mem) → void *
│   ├── 遍历 libc.tls_head 链表，逐模块调用 memcpy 复制 TLS 初始映像
│   ├── 构造 DTV (Dynamic Thread Vector) 数组
│   ├── 依赖: TLS_ABOVE_TP / GAP_ABOVE_TP / DTP_OFFSET / TP_OFFSET (架空构宏)
│   ├── 依赖: struct tls_module (libc.h), struct pthread (pthread_impl.h)
│   └── 返回: pthread_t (指向 struct pthread 起始地址)
│
├── __init_tp(void *p) → int
│   ├── 初始化 struct pthread 字段: self, detach_state, tid, locale, robust_list, sysinfo, prev/next
│   ├── 调用 __set_thread_area(TP_ADJ(p)) 设置线程指针寄存器
│   ├── 调用 __syscall(SYS_set_tid_address, &__thread_list_lock) 设置 TID 清除地址
│   ├── 成功 → 置 libc.can_do_threads = 1，返回 0
│   └── 失败 → 返回 -1
│
└── a_crash() [atomic.h] — TLS 初始化失败时终止进程
```

### 跨文件依赖速查

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `struct pthread` / `pthread_t` | `pthread_impl.h` | 已有 spec: `src/internal/spec/pthread_impl.md` |
| `struct tls_module` / `struct __libc` / `libc` | `libc.h` | 已有 spec: `src/internal/spec/libc.md` |
| `__syscall()`, `SYS_*` | `syscall.h` | 外部（Linux 系统调用层），跳过 |
| `memcpy` | `<string.h>` | 外部（C 标准库），跳过 |
| `TP_ADJ`, `TLS_ABOVE_TP`, `DTP_OFFSET` | `pthread_arch.h` (架空构) | 架空构依赖，见下文架空构差异表 |
| `__set_thread_area(void*)` | `src/thread/__set_thread_area.c` | 跨文件，见该文件 spec |
| `a_crash()` | `atomic.h` | 外部（原子操作层），跳过 |
| `__default_stacksize` / `DEFAULT_STACK_MAX` | `pthread_impl.h` | 已有 spec |
| `__sysinfo` | `libc.h` (定义于 `src/internal/defsysinfo.c`) | 已有 spec |
| `_DYNAMIC[]` | ELF 动态链接器导出 | 外部（弱符号），跳过 |
| `Phdr` / `Elf32_Phdr` / `Elf64_Phdr` | `<elf.h>` | 外部（ELF 标准类型），跳过 |
| `offsetof` | `<stddef.h>` | 外部（C 标准），跳过 |

---

## 架空构差异表

`__init_tls.c` 的实现根据架构不同体现为三组条件编译：

| 宏 | x86_64 | aarch64 | i386 | arm | 定义来源 |
|----|--------|---------|------|-----|----------|
| `TLS_ABOVE_TP` | 未定义 (TLS below TP) | 已定义 | 未定义 | 已定义 | `arch/<arch>/pthread_arch.h` |
| `GAP_ABOVE_TP` | N/A | 16 | N/A | 8 | 同上 |
| `TP_OFFSET` | 0 (默认) | 0 (默认) | 0 (默认) | 0 (默认) | `pthread_impl.h` 默认值 |
| `DTP_OFFSET` | 0 (默认) | 0 (默认) | 0 (默认) | 0 (默认) | 同上 |
| `Phdr` typedef | `Elf64_Phdr` (ULONG_MAX > 32-bit) | `Elf64_Phdr` | `Elf32_Phdr` | `Elf32_Phdr` | 本文件根据 `ULONG_MAX` 选择 |

### 内存布局差异 (关键)

**TLS Below TP (x86_64, i386)**:
```
低地址 → [DTV] [TLS 数据 (main_tls.offset 处)] [pthread 结构体] [TLS 数据续] ← 高地址
         ↑                                                       ↑
      dtv 指针                                             TP (thread pointer)
                 main_tls.offset == main_tls.size
```

**TLS Above TP (aarch64, arm, riscv64 等)**:
```
低地址 → [pthread 结构体] [GAP_ABOVE_TP] [TLS 数据 (main_tls.offset 处)] [DTV] ← 高地址
         ↑                                                 ↑
     td = pthread_t                                    TP (thread pointer)
                 main_tls.offset == GAP_ABOVE_TP + 对齐填充
```

---

## 内部结构体与常量 (本文件定义)

### `struct builtin_tls` (内部静态变量)

```c
static struct builtin_tls {
    char c;
    struct pthread pt;
    void *space[16];
} builtin_tls[1];
```

[Visibility]: Internal (不导出) — 仅在本编译单元内使用，对小型 TLS 程序提供栈上/静态分配的 TCB + TLS 存储

**Intent**: 为 TLS 数据量较小的程序提供内联存储，避免系统调用 `mmap` 的开销。`builtin_tls[1]` 是文件级静态变量（生命周期 = 整个进程），程序启动时若 `libc.tls_size <= sizeof(builtin_tls)` 则直接使用此缓冲区。

**字段说明**:
- `c` (`char`): 仅用于对齐填充，无实际语义。确保 `offsetof(struct builtin_tls, pt)` 返回 `struct pthread` 在结构体内的对齐偏移。
- `pt` (`struct pthread`): 主线程的线程控制块（TCB），大小为 `sizeof(struct pthread)`。
- `space[16]` (`void *[16]`): TLS 数据存储空间，大小为 `16 * sizeof(void *)`（x86_64 上为 128 字节）。

**不变量**: `builtin_tls` 始终存在（静态存储期），但仅在 `libc.tls_size <= sizeof(builtin_tls)` 时被实际使用。若 TLS 需求超过此大小，`static_init_tls` 改为 `mmap` 分配。

---

### `MIN_TLS_ALIGN` 宏

```c
#define MIN_TLS_ALIGN offsetof(struct builtin_tls, pt)
```

[Visibility]: Internal (不导出) — 编译时常量，仅本文件内使用

**Intent**: 保证 `struct pthread` 在其所在内存块内的对齐至少为 `MIN_TLS_ALIGN` 字节。因为在 `TLS_ABOVE_TP` 模式下，`struct pthread` 紧挨着 TLS 数据块放置，需要确保 pthread 结构体本身的对齐不受 TLS 数据对齐影响。

**值**: `offsetof(struct builtin_tls, pt)` — 即 `struct pthread` 在 `builtin_tls` 中的起始偏移。在 x86_64 上，由于 `struct pthread` 包含 `uintptr_t` 等字段，其自然对齐通常为 8 或 16 字节。

---

### `main_tls` (内部静态变量)

```c
static struct tls_module main_tls;
```

[Visibility]: Internal (不导出) — 主程序（可执行文件）的 TLS 模块描述符，仅本文件内使用

**字段初始化时序** (由 `static_init_tls` 完成):
1. `image` ← ELF PT_TLS 段在内存中的实际地址 (`base + tls_phdr->p_vaddr`)
2. `len` ← `tls_phdr->p_filesz` (TLS 数据在 ELF 文件中的大小)
3. `size` ← `tls_phdr->p_memsz` (TLS 数据在内存中的大小，含 .tbss)
4. `align` ← `tls_phdr->p_align`，但至少为 `MIN_TLS_ALIGN`
5. `offset` ← 根据 TLS 布局模式计算 (见 `static_init_tls`)
6. `next` ← `NULL` (主模块，链表尾部)
7. 此模块被挂接到 `libc.tls_head` 链表头部

---

## 函数规约（按拓扑顺序）

---

### `__init_tp` (内部函数)

```c
int __init_tp(void *p);
```

```rust
fn __init_tp(p: *mut c_void) -> c_int;
```

[Visibility]: Internal (不导出) — musl 内部线程指针初始化函数，在 `pthread_impl.h` 中声明为 `hidden`，POSIX/C 标准未定义

**Intent**: 完成线程控制块 (TCB) 的关键字段初始化，设置硬件线程指针寄存器（通过 `__set_thread_area`），并向内核注册 TID 清除地址。此函数在主线程创建和子线程创建（`pthread_create`）时均被调用。

**前置条件**:
- `p` 指向的内存区域至少为 `sizeof(struct pthread)` 字节，且满足 `struct pthread` 的对齐要求
- 进程尚未完成线程指针初始化（或正为新线程初始化独立的 TCB）
- 调用时处于单线程环境（主线程）或调用者持有相关锁（子线程）
- `libc.global_locale` 已初始化
- `__sysinfo` 已被设置（若架构使用 vDSO）

**后置条件**:

Case 1 — 成功 (返回 0):
- `td->self == td`（TCB 的 self 指针指向自身）
- `td->detach_state == DT_JOINABLE`
- `td->tid` 被设置为当前线程的内核 TID，内核被告知在 TID 被释放时将 `&__thread_list_lock` 清零（通过 `SYS_set_tid_address`）
- `td->locale == &libc.global_locale`（指向全局 C locale）
- `td->robust_list.head == &td->robust_list.head`（robust mutex 链表初始化为自环空表）
- `td->sysinfo == __sysinfo`（vDSO sysinfo 地址拷贝到 TCB，供 `__syscall` 汇编路径使用）
- `td->next == td->prev == td`（线程链表初始化为自环，尚未链接到全局线程列表）
- 硬件线程指针寄存器已设置（`__set_thread_area(TP_ADJ(p))` 返回 0），`__get_tp()` 现在可用
- `libc.can_do_threads = 1`（仅在 `__set_thread_area` 返回 0 时 — 意味着架构支持线程指针设置）

Case 2 — 失败 (返回 -1):
- `__set_thread_area()` 返回负值（如架构不支持 `SYS_set_thread_area`）
- `libc.can_do_threads` 未被修改（仍为 0）
- 调用者应将其视为致命错误（`static_init_tls` 会调用 `a_crash()`）

**系统调用**:
- `__set_thread_area(TP_ADJ(p))`: 设置线程指针寄存器（x86_64 上为 `arch_prctl(ARCH_SET_FS, ...)`），目标地址 = TCB 末尾（TLS Below TP）或 TCB + sizeof(struct pthread)（TLS Above TP）
- `__syscall(SYS_set_tid_address, &__thread_list_lock)`: 注册 clear_child_tid 地址，在线程退出时由内核原子性地将该地址清零并执行 `futex(FUTEX_WAKE)`。musl 使用 `__thread_list_lock` 的地址而非独立变量作为优化

**不变量**:
- `td->self` 在整个线程生命周期中不变，始终等于 `td` 的起始地址
- `td->robust_list.head` 初始为自环是 POSIX robust mutex 协议的要求

**架空构差异**:
- `TP_ADJ(p)` 在 TLS Below TP (x86_64) 上为 `(char*)p`，即 TP = TCB 起始
- `TP_ADJ(p)` 在 TLS Above TP (aarch64) 上为 `(char*)p + sizeof(struct pthread) + TP_OFFSET`，即 TP = TCB 末尾
- `SYS_set_thread_area` 在 x86_64 上实际映射为 `arch_prctl` 系统调用

---

### `__copy_tls` (内部函数)

```c
void *__copy_tls(unsigned char *mem);
```

```rust
fn __copy_tls(mem: *mut u8) -> *mut c_void;
```

[Visibility]: Internal (不导出) — musl 内部 TLS 初始化函数，在 `pthread_impl.h` 中声明为 `hidden`，POSIX/C 标准未定义

**Intent**: 将链接时注册的所有 TLS 模块的初始数据映像复制到指定内存区域，构造 DTV (Dynamic Thread Vector) 数组，并返回指向已初始化的 `struct pthread` 的指针。此函数同时用于主线程初始化（`__init_tls`）和新线程创建（`pthread_create` 的 `__clone` 回调）。

**前置条件**:
- `mem` 指向已分配的内存区域，大小至少为 `libc.tls_size` 字节
- `libc` 结构体的以下字段已正确初始化：
  - `tls_head`: 指向 TLS 模块链表（至少包含 `main_tls`）
  - `tls_cnt`: TLS 模块数量
  - `tls_size`: TLS 总大小
  - `tls_align`: TLS 对齐要求
- `libc.tls_head` 链表中每个模块的 `image` / `len` 已初始化
- 调用时尚未有其他线程访问同一 `mem` 区域（无竞争）

**后置条件 (Case 1 — TLS Below TP, 如 x86_64)**:
- `dtv` (`uintptr_t *`) 位于 `mem` 起始处
- `dtv[0] = libc.tls_cnt`（DTV 长度）
- 对每个 TLS 模块 `p` (i = 1, 2, ..., tls_cnt):
  - `dtv[i] = (uintptr_t)(mem + p->offset) + DTP_OFFSET`（DTV 条目指向 TLS 块在内核中的偏移地址）
  - `memcpy(mem + p->offset, p->image, p->len)` 已执行（TLS 初始数据已就位）
- `struct pthread` 位于 `mem + libc.tls_size - sizeof(struct pthread)`，按 `tls_align` 对齐
- `td->dtv = dtv` 已设置
- 返回 `td`（指向 `struct pthread` 的指针）

**后置条件 (Case 2 — TLS Above TP, 如 aarch64)**:
- `dtv` (`uintptr_t *`) 位于 `mem + libc.tls_size - (tls_cnt + 1) * sizeof(uintptr_t)`
- 其他逻辑相同，但 TLS 数据和 `struct pthread` 的相对位置相反：
  - `struct pthread` 起始地址 = `mem`（按 `tls_align` 对齐）
  - TLS 数据位于 `mem + p->offset`（在 `struct pthread` 之上）
  - `dtv[i] = (uintptr_t)(mem + p->offset) + DTP_OFFSET`

**后置条件 (共同)**:
- 所有 TLS 模块的 `.tbss` 段（`size - len` 部分）保持为零（由 `mem` 的分配方式保证——`mmap` 返回零页或 `builtin_tls` 静态零初始化）
- DTV 数组、TLS 数据、`struct pthread` 均在同一个 `libc.tls_size` 大小的内存块中
- 返回值指向的 `struct pthread` 是完整初始化的 TCB（但部分字段尚未设置，需由 `__init_tp` 完成）

**系统算法**:
1. 根据 `TLS_ABOVE_TP` 宏选择两种布局之一
2. 计算 DTV 数组位置（位于内存块的一端）
3. 计算 `struct pthread` 的位置（位于内存块的另一端，按 `tls_align` 对齐）
4. `td->dtv = dtv` — DTV 指针写入 TCB
5. 遍历 `libc.tls_head` 链表，对每个模块执行 `memcpy` 复制初始化映像
6. 设置 `dtv[0] = libc.tls_cnt`

---

### `static_init_tls` / `__init_tls` (内部函数)

```c
// 内部静态函数，通过 weak_alias 暴露给 musl 内部其他模块
static void static_init_tls(size_t *aux);
hidden void __init_tls(size_t *aux);  // weak_alias(static_init_tls, __init_tls)
```

```rust
fn __init_tls(aux: *mut usize);
```

[Visibility]: Internal (不导出) — musl 运行时初始化函数，在 `libc.h` 中声明为 `hidden`，由 `__libc_start_main` 调用。用户程序不可直接调用，POSIX/C 标准未定义

**Intent**: 进程启动时的一次性 TLS 初始化。解析 ELF 辅助向量 (aux vector) 中的程序头信息，定位 PT_TLS 段，计算 TLS 模块布局，分配 TCB + TLS 内存，并完成主线程的 TLS 初始化。调用后 `__pthread_self()` 可正常工作，`errno` / `locale` 等 TLS 变量可用。

**前置条件**:
- `aux` 指向辅助向量数组，其中 `aux[AT_PHDR]`、`aux[AT_PHNUM]`、`aux[AT_PHENT]` 已由内核/动态链接器填充
- 程序尚未初始化 TLS（`libc.tls_head == NULL`，`libc.tls_cnt == 0`）
- 这是进程启动后对 `__init_tls` 的第一次调用（不可重入）
- `__default_stacksize` 已被初始化为 `DEFAULT_STACK_SIZE`

**后置条件 (Case 1 — 正常路径)**:
1. **ELF 解析阶段**: `main_tls` 的 `image`、`len`、`size`、`align` 从 PT_TLS 程序头中提取；`libc.tls_head = &main_tls`；`libc.tls_cnt = 1`
2. **栈大小调整**: 若存在 `PT_GNU_STACK` 段且其 `p_memsz > __default_stacksize`，则 `__default_stacksize` 更新为 `p_memsz`（但不超过 `DEFAULT_STACK_MAX` = 8MB）
3. **TLS 布局计算**:
   - `main_tls.size` 向上对齐至 `main_tls.align`
   - `main_tls.offset` 计算：
     - TLS Below TP: `main_tls.offset = main_tls.size`
     - TLS Above TP: `main_tls.offset = GAP_ABOVE_TP`，再按 `main_tls.align` 对齐
   - `main_tls.align` = `max(main_tls.align, MIN_TLS_ALIGN)`
   - `libc.tls_align = main_tls.align`
   - `libc.tls_size = 2 * sizeof(void*) + sizeof(struct pthread) + main_tls.offset + main_tls.size + main_tls.align`（再按 `MIN_TLS_ALIGN` 对齐）
     - 其中 `+ main_tls.offset` 仅在 `TLS_ABOVE_TP` 时参加计算（`#ifdef`）
4. **内存分配**:
   - 若 `libc.tls_size <= sizeof(builtin_tls)`: 使用静态缓冲区 `builtin_tls`
   - 否则: `__syscall(SYS_mmap2/SYS_mmap, 0, libc.tls_size, PROT_READ|PROT_WRITE, MAP_ANONYMOUS|MAP_PRIVATE, -1, 0)`
5. **TLS 复制**: 调用 `__copy_tls(mem)`，返回初始化后的 `pthread_t td`
6. **TCB 初始化**: 调用 `__init_tp(td)`
   - 若返回 < 0: **进程终止** (`a_crash()`)
   - 若返回 == 0: 主线程 TLS 完全就绪

**后置条件 (Case 2 — 无 PT_TLS 段)**:
- 若 ELF 遍历后 `tls_phdr == NULL`（程序无 TLS 数据），则 `main_tls` 保持零初始化状态，但 `libc.tls_align`、`libc.tls_size`、`libc.tls_cnt`、`libc.tls_head` 均未被设置
- 后续 `__copy_tls` 和 `__init_tp` 仍被调用，但由于 `libc.tls_cnt == 0`，循环体不执行
- 实际效果：主线程的 TCB 被分配和初始化（含 thread pointer），但 TLS 变量不可用

**全局变量修改**:
- `libc.tls_head`, `libc.tls_size`, `libc.tls_align`, `libc.tls_cnt` — TLS 运行时状态
- `libc.can_do_threads` — 通过 `__init_tp` 间接设置
- `__default_stacksize` — 根据 `PT_GNU_STACK` 可能被更新
- `__thread_list_lock` — 被 `__init_tp` 作为 `SYS_set_tid_address` 的目标地址写入

**弱符号依赖**:
- `_DYNAMIC[]`: ELF 动态段地址，用于计算加载基址（当 PT_PHDR 的 `p_vaddr` 不可靠时，优先使用 `_DYNAMIC` 来计算 base）。若动态链接器未定义此符号（静态链接），其值为 0，此时回退到纯 PT_PHDR 方式计算 base。

**系统调用**:
- `SYS_mmap2` (或 `SYS_mmap`): 仅当 `builtin_tls` 不足时调用
- 通过 `__copy_tls` → 无系统调用
- 通过 `__init_tp` → `SYS_set_thread_area` 或 `arch_prctl(ARCH_SET_FS)` + `SYS_set_tid_address`

**不变量**:
- **I1**: `libc.tls_align` 始终 ≥ `MIN_TLS_ALIGN`（保证 `struct pthread` 对齐）
- **I2**: `libc.tls_size` 始终 ≥ `2 * sizeof(void*) + sizeof(struct pthread) + main_tls.size + main_tls.align`（保证 DTV + TCB + TLS 数据 + 对齐填充的最小空间）
- **I3**: 初始化成功后，`__pthread_self()` 可通过 `__get_tp()` 获得有效的 `pthread_t`
- **I4**: `main_tls.next == NULL`（主模块位于链表末尾）

**致命错误条件**:
- `mem` 分配成功但 `__init_tp` 失败 → `a_crash()`（写入地址 0 触发 SIGSEGV）
- 注释明确指出：`mmap` 返回的 `(void*)(-4095..-1)`（即错误码）会因解引用而 crash，无需显式检查

---

## 全局变量

### `__thread_list_lock`

```c
volatile int __thread_list_lock;
```

[Visibility]: Internal (不导出) — musl 线程系统内部全局锁，在 `pthread_impl.h` 中声明为 `extern hidden`，用户程序不可访问

**Intent**: 保护全局线程链表 (`struct pthread` 的 `prev`/`next` 双向链表) 的互斥锁。同时，其地址被用作 `SYS_set_tid_address` 的 `clear_child_tid` 参数——当线程退出时，内核零化此地址的值并通过 `futex(FUTEX_WAKE)` 唤醒等待者。

**双重用途**:
1. **作为锁值**: `volatile int`，由 `__tl_lock()` / `__tl_unlock()` 通过 `a_cas` 操作实现互斥
2. **作为 TID 清除地址**: 其地址在 `__init_tp` 中传递给 `SYS_set_tid_address`，作为线程退出时的通知机制

**初始化**: 声明时为零初始化（BSS 段），`__init_tp` 将其地址传递给内核。

---

## 调用时序（进程启动流程）

```
_start (crt1.o / Scrt1.o)
  → __libc_start_main (src/env/__libc_start_main.c)
    → __init_libc(envp, pn)     ← 解析 auxv，设置 libc.page_size, __hwcap, __sysinfo, __progname
    → __init_tls(aux)            ← ★ 本文件定义的函数 ★
      → static_init_tls(aux)     ← 解析 ELF PHDR, 计算 TLS 布局, 分配内存
        → __copy_tls(mem)        ← 复制 TLS 初始数据 + 构造 DTV
        → __init_tp(td)          ← 设置线程指针寄存器 + 初始化 TCB
    → __init_ssp(aux[AT_RANDOM]) ← 初始化栈保护 Canary
    → (...安全加固检查, 预初始化 fd...)
    → __libc_start_init()        ← 调用 .init / .init_array
    → main(argc, argv, envp)     ← 用户入口
```

---

## 架空构实现要点（rusl 参考）

1. **TLS 布局二选一**: 必须根据目标架构在编译期选择 TLS Below TP 或 TLS Above TP 布局。这不能是运行时分支。
2. **builtin_tls 替代方案**: Rust 可以用静态 `MaybeUninit<[u8; N]>` 替代 `builtin_tls`，但需要确保对齐满足 `struct pthread` 的要求。
3. **__copy_tls 中的 memcpy**: Rust 中应使用 `ptr::copy_nonoverlapping` 代替 `memcpy`。
4. **__init_tp 中的系统调用**: Rust 中需使用 `syscall!` 宏或 `libc` crate 的 `syscall` 函数（注意 `no_std` 约束）。
5. **ELF 解析**: 可以复用 `goblin` crate（支持 `no_std`）来解析 ELF program headers，避免手动解析 `Phdr` 结构。
6. **a_crash 替代**: Rust 中可用 `core::intrinsics::abort()` 或直接 `core::hint::unreachable_unchecked()` 但出于安全考虑，推荐 `panic!("TLS init failed")`。