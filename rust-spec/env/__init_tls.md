# __init_tls — Rust 接口归约

> **原始 C spec**: `src/env/spec/__init_tls.md`
> **来源文件**: `musl/src/env/__init_tls.c`
> **复杂度层级**: Level 3 — 高度优化设计（ELF 程序头解析 + TLS 布局计算 + 线程控制块初始化 + 条件编译架构差异）
> **调用者**: `__libc_start_main()` (rusl `env/__libc_start_main` 模块)，在进程启动早期调用，此时单线程、无锁竞争

---

## 依赖图

```
init_tls 模块
├── [ELF 程序头解析] 依赖 Phdr / Elf64_Phdr 类型 (elf 模块)
│   ├── 读取 auxv[AT_PHDR], auxv[AT_PHNUM], auxv[AT_PHENT]
│   └── 遍历 program headers 寻找 PT_PHDR, PT_DYNAMIC, PT_TLS, PT_GNU_STACK
│
├── [main_tls 布局计算] 依赖 crate::internal::libc::TlsModule
│   ├── 读取 tls_phdr->p_vaddr, p_filesz, p_memsz, p_align
│   ├── 计算 BUILTIN_TLS_SIZE (向上对齐)
│   ├── 计算 main_tls.offset (TLS_ABOVE_TP / !TLS_ABOVE_TP 两种模式)
│   ├── 更新 libc.tls_align, libc.tls_size, libc.tls_cnt, libc.tls_head
│   └── 依赖 MIN_TLS_ALIGN 常量 (本模块定义)、BUILTIN_TLS 静态缓冲区
│
├── [内存分配]
│   ├── 若 libc.tls_size ≤ BUILTIN_TLS_SIZE → 使用 BUILTIN_TLS 静态缓冲区
│   └── 否则 → syscall::mmap(...) 匿名映射
│
├── copy_tls(mem: *mut u8) -> *mut c_void
│   ├── 遍历 libc.tls_head 链表，逐模块调用 ptr::copy_nonoverlapping 拷贝 TLS 初始映像
│   ├── 构造 DTV (Dynamic Thread Vector) 数组
│   ├── 依赖: TLS_ABOVE_TP / GAP_ABOVE_TP / DTP_OFFSET / TP_OFFSET (架构宏)
│   ├── 依赖: crate::internal::libc::TlsModule, crate::internal::pthread_impl::Pthread
│   └── 返回: *mut Pthread (指向 struct Pthread 起始地址)
│
├── init_tp(p: *mut c_void) -> c_int
│   ├── 初始化 struct Pthread 字段: self_, detach_state, tid, locale, robust_list, sysinfo, prev/next
│   ├── 调用 set_thread_area(TP_ADJ(p)) 设置线程指针寄存器
│   ├── 调用 syscall::set_tid_address(&THREAD_LIST_LOCK) 设置 TID 清除地址
│   ├── 成功 → 置 libc.can_do_threads = 1，返回 0
│   └── 失败 → 返回 -1
│
└── 若 init_tp 失败 → core::intrinsics::abort() — TLS 初始化失败时终止进程
```

---

### 跨文件依赖速查

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `Pthread` / `*mut Pthread` | `crate::internal::pthread_impl` | 已有 spec: `src/internal/rust-spec/pthread_impl.md` |
| `TlsModule` / `Libc` / `libc` | `crate::internal::libc` | 已有 spec: `src/internal/rust-spec/libc.md` |
| `syscall::mmap`, `syscall::set_tid_address` 等 | `crate::syscall` (rusl 内部) | 系统调用封装 |
| `ptr::copy_nonoverlapping` | `core::ptr` | Rust core 内存操作 |
| `TP_ADJ`, `TLS_ABOVE_TP`, `DTP_OFFSET`, `GAP_ABOVE_TP` | `crate::internal::pthread_arch` (架构相关) | 架构依赖，见下文架构差异表 |
| `set_thread_area(p: *mut c_void) -> c_int` | `crate::internal::pthread_impl` | 已有 spec |
| `THREAD_LIST_LOCK` | `crate::internal::pthread_impl` | 已有 spec (原 `__thread_list_lock`) |
| `DEFAULT_STACK_MAX` | `crate::internal::pthread_impl` | 已有 spec |
| `__sysinfo` | `crate::internal::libc::defsysinfo` | 已有 spec |
| `_DYNAMIC[]` | ELF 动态链接器导出 | 外部（弱符号），跳过 |
| `Phdr` / `Elf32_Phdr` / `Elf64_Phdr` | `crate::elf` (rusl 内部或 `elf` crate) | ELF 标准类型 |

---

## 架构差异表

`init_tls` 模块的实现根据架构不同体现为三组条件编译：

| 宏/常量 | x86_64 | aarch64 | i386 | arm | 定义来源 |
|--------|--------|---------|------|-----|----------|
| `TLS_ABOVE_TP` | `false` (TLS below TP) | `true` | `false` | `true` | `crate::internal::pthread_arch` |
| `GAP_ABOVE_TP` | N/A | 16 | N/A | 8 | 同上 |
| `TP_OFFSET` | 0 (默认) | 0 (默认) | 0 (默认) | 0 (默认) | `crate::internal::pthread_arch` 默认值 |
| `DTP_OFFSET` | 0 (默认) | 0 (默认) | 0 (默认) | 0 (默认) | 同上 |
| `Phdr` 类型 | `Elf64_Phdr` | `Elf64_Phdr` | `Elf32_Phdr` | `Elf32_Phdr` | 根据 `target_pointer_width` 选择 |

**Rust 实现**: 使用 `#[cfg(target_arch = "x86_64")]` 等条件编译属性在编译期选择正确的布局。

### 内存布局差异 (关键)

**TLS Below TP (x86_64, i386)**:
```
低地址 → [DTV] [TLS 数据 (main_tls.offset 处)] [Pthread 结构体] [TLS 数据续] ← 高地址
         ↑                                                       ↑
      DTV 指针                                             TP (thread pointer)
                 main_tls.offset == main_tls.size
```

**TLS Above TP (aarch64, arm, riscv64 等)**:
```
低地址 → [Pthread 结构体] [GAP_ABOVE_TP] [TLS 数据 (main_tls.offset 处)] [DTV] ← 高地址
         ↑                                                 ↑
     td = *mut Pthread                                TP (thread pointer)
                 main_tls.offset == GAP_ABOVE_TP + 对齐填充
```

---

## 模块内部结构体与常量

### `BuiltinTls` — 内联 TLS 存储

```rust
// Rust 签名
#[repr(C)]
struct BuiltinTls {
    c: u8,                          // 对齐占位符，确保 Pthread 在结构体内按自然对齐放置
    pt: MaybeUninit<Pthread>,        // 主线程的线程控制块 (TCB)
    space: [usize; 16],             // TLS 数据存储空间（16 words）
}
```

[Visibility]: `pub(self)` — 仅本模块内使用，对小型 TLS 程序提供静态分配的 TCB + TLS 存储

**意图**: 为 TLS 数据量较小的程序提供内联存储，避免系统调用 `mmap` 的开销。当 `libc.tls_size <= size_of::<BuiltinTls>()` 时直接使用此缓冲区。

**C 对照**: 替代 C 的 `static struct builtin_tls { char c; struct pthread pt; void *space[16]; } builtin_tls[1];`

**不变量**:
- `BuiltinTls` 始终存在（静态存储期），但仅在 `libc.tls_size <= size_of::<BuiltinTls>()` 时被实际使用
- `pt` 字段的偏移量（`offset_of!(BuiltinTls, pt)`）等于 `align_of::<Pthread>()`，确保 pthread 结构体正确对齐

---

### `BUILTIN_TLS` — 静态缓冲区

```rust
// Rust 签名
static BUILTIN_TLS: UnsafeCell<BuiltinTls> = UnsafeCell::new(BuiltinTls {
    c: 0,
    pt: MaybeUninit::uninit(),
    space: [0; 16],
});
```

[Visibility]: `pub(self)` — 模块内部静态变量，进程生命周期内存在

**设计说明**: 使用 `UnsafeCell<BuiltinTls>` 替代 C 的 `static builtin_tls[1]`。`UnsafeCell` 允许通过不可变引用获取可变裸指针，符合 Rust 的安全性要求；同时避免了 `static mut` 的安全警告。

---

### `BUILTIN_TLS_SIZE` 常量

```rust
// Rust 签名
const BUILTIN_TLS_SIZE: usize = size_of::<BuiltinTls>();
```

[Visibility]: `pub(self)` — 编译时常量，仅本模块内使用

**意图**: 表达内置 TLS 缓冲区的总容量。当 `libc.tls_size > BUILTIN_TLS_SIZE` 时回退到 `mmap` 分配。

---

### `MIN_TLS_ALIGN` 常量

```rust
// Rust 签名
const MIN_TLS_ALIGN: usize = offset_of!(BuiltinTls, pt);
```

[Visibility]: `pub(self)` — 编译时常量，仅本模块内使用

**意图**: 保证 `Pthread` 在其所在内存块内的对齐至少为 `MIN_TLS_ALIGN` 字节。在 `TLS_ABOVE_TP` 模式下，`Pthread` 紧挨着 TLS 数据块放置，需要确保 Pthread 结构体本身的对齐不受 TLS 数据对齐影响。

**值**: `offset_of!(BuiltinTls, pt)` — 即 Pthread 在 BuiltinTls 中的起始偏移，等价于 `align_of::<Pthread>()`。

---

### `MAIN_TLS` — 主程序 TLS 模块描述符

```rust
// Rust 签名
static MAIN_TLS: UnsafeCell<TlsModule> = UnsafeCell::new(TlsModule {
    next: core::ptr::null_mut(),
    image: core::ptr::null(),
    len: 0,
    size: 0,
    align: 0,
    offset: 0,
});
```

[Visibility]: `pub(self)` — 主程序（可执行文件）的 TLS 模块描述符，仅本模块内使用

**C 对照**: 替代 C 的 `static struct tls_module main_tls;`

**字段初始化时序** (由 `init_tls` 完成):
1. `image` ← ELF PT_TLS 段在内存中的实际地址 (`base + tls_phdr->p_vaddr`)
2. `len` ← `tls_phdr->p_filesz` (TLS 数据在 ELF 文件中的大小)
3. `size` ← `tls_phdr->p_memsz` (TLS 数据在内存中的大小，含 .tbss)
4. `align` ← `tls_phdr->p_align`，但至少为 `MIN_TLS_ALIGN`
5. `offset` ← 根据 TLS 布局模式计算 (见 `init_tls`)
6. 此模块被挂接到 `libc.tls_head` 链表头部

---

## 函数规约（按拓扑顺序）

---

### `init_tp` — 初始化线程指针

```rust
// Rust 签名
pub(crate) fn init_tp(p: *mut core::ffi::c_void) -> core::ffi::c_int;
```

[Visibility]: Internal (`pub(crate)`) — rusl 内部线程指针初始化函数，POSIX/C 标准未定义

**C 对照**: `int __init_tp(void *p);` (`pthread_impl.h` 中声明为 `hidden`)

**意图**: 完成线程控制块 (TCB) 的关键字段初始化，设置硬件线程指针寄存器（通过 `set_thread_area`），并向内核注册 TID 清除地址。此函数在主线程创建和子线程创建（`pthread_create`）时均被调用。

**前置条件**:
- `p` 指向的内存区域至少为 `size_of::<Pthread>()` 字节，且满足 `Pthread` 的对齐要求
- 进程尚未完成线程指针初始化（或正为新线程初始化独立的 TCB）
- 调用时处于单线程环境（主线程）或调用者持有相关锁（子线程）
- `libc.global_locale` 已初始化
- `__sysinfo` 已被设置（若架构使用 vDSO）

**后置条件**:

Case 1 — 成功 (返回 0):
- `td.self_ == td`（TCB 的 self_ 指针指向自身）
- `td.detach_state == DetachState::Joinable`
- `td.tid` 被设置为当前线程的内核 TID，内核被告知在 TID 被释放时将 `&THREAD_LIST_LOCK` 清零（通过 `SYS_set_tid_address`）
- `td.locale == &libc.global_locale`（指向全局 C locale）
- `td.robust_list.head == &td.robust_list.head`（robust mutex 链表初始化为自环空表）
- `td.sysinfo == __sysinfo`（vDSO sysinfo 地址拷贝到 TCB，供 syscall 汇编路径使用）
- `td.prev == td.next == td`（线程链表初始化为自环，尚未链接到全局线程列表）
- 硬件线程指针寄存器已设置（`set_thread_area(TP_ADJ(p))` 返回 0），`__get_tp()` 现在可用
- `libc.can_do_threads = true`（仅在 `set_thread_area` 返回 0 时 — 意味着架构支持线程指针设置）

Case 2 — 失败 (返回 -1):
- `set_thread_area()` 返回负值（如架构不支持 `SYS_set_thread_area`）
- `libc.can_do_threads` 未被修改（仍为 false）
- 调用者应将其视为致命错误（`init_tls` 会调用 `core::intrinsics::abort()`）

**系统调用**:
- `set_thread_area(TP_ADJ(p))`: 设置线程指针寄存器（x86_64 上为 `arch_prctl(ARCH_SET_FS, ...)`），目标地址 = TCB 末尾（TLS Below TP）或 TCB + size_of::<Pthread>()（TLS Above TP）
- `syscall::set_tid_address(&THREAD_LIST_LOCK)`: 注册 clear_child_tid 地址，在线程退出时由内核原子性地将该地址清零并执行 `futex(FUTEX_WAKE)`。rusl 使用 `THREAD_LIST_LOCK` 的地址而非独立变量作为优化

**不变量**:
- `td.self_` 在整个线程生命周期中不变，始终等于 `td` 的起始地址
- `td.robust_list.head` 初始为自环是 POSIX robust mutex 协议的要求

**架构差异**:
- `TP_ADJ(p)` 在 TLS Below TP (x86_64) 上为 `p as *mut u8`，即 TP = TCB 起始
- `TP_ADJ(p)` 在 TLS Above TP (aarch64) 上为 `((p as *mut u8).add(size_of::<Pthread>())).add(TP_OFFSET)`，即 TP = TCB 末尾 + TP_OFFSET
- `SYS_set_thread_area` 在 x86_64 上实际映射为 `arch_prctl` 系统调用

**Rust 实现要点**: 内部通过 `unsafe` 块操作裸指针访问 `Pthread` 字段。`detach_state` 使用 `AtomicI32` 的 `store` 方法替代 C 的直接赋值，提供明确定义的内存顺序。

---

### `copy_tls` — 复制 TLS 初始数据

```rust
// Rust 签名
pub(crate) fn copy_tls(mem: *mut u8) -> *mut core::ffi::c_void;
```

[Visibility]: Internal (`pub(crate)`) — rusl 内部 TLS 初始化函数，POSIX/C 标准未定义

**C 对照**: `void *__copy_tls(unsigned char *mem);` (`pthread_impl.h` 中声明为 `hidden`)

**意图**: 将链接时注册的所有 TLS 模块的初始数据映像复制到指定内存区域，构造 DTV (Dynamic Thread Vector) 数组，并返回指向已初始化的 `Pthread` 的指针。此函数同时用于主线程初始化（`init_tls`）和新线程创建（`pthread_create` 的 `clone` 回调）。

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
- `dtv` (`*mut usize`) 位于 `mem` 起始处
- `dtv[0] = libc.tls_cnt`（DTV 长度）
- 对每个 TLS 模块 `p` (i = 1, 2, ..., tls_cnt):
  - `dtv[i] = (mem.add(p.offset) as usize) + DTP_OFFSET`（DTV 条目指向 TLS 块在内核中的偏移地址）
  - `ptr::copy_nonoverlapping(p.image, mem.add(p.offset), p.len)` 已执行（TLS 初始数据已就位）
- `Pthread` 位于 `mem.add(libc.tls_size - size_of::<Pthread>())`，按 `tls_align` 对齐
- `td.dtv = dtv` 已设置
- 返回 `td`（指向 Pthread 的指针）

**后置条件 (Case 2 — TLS Above TP, 如 aarch64)**:
- `dtv` (`*mut usize`) 位于 `mem.add(libc.tls_size - (tls_cnt + 1) * size_of::<usize>())`
- 其他逻辑相同，但 TLS 数据和 Pthread 的相对位置相反：
  - Pthread 起始地址 = `mem`（按 `tls_align` 对齐）
  - TLS 数据位于 `mem.add(p.offset)`（在 Pthread 之上）
  - `dtv[i] = (mem.add(p.offset) as usize) + DTP_OFFSET`

**后置条件 (共同)**:
- 所有 TLS 模块的 `.tbss` 段（`size - len` 部分）保持为零（由 `mem` 的分配方式保证——`mmap` 返回零页或 `BUILTIN_TLS` 静态零初始化）
- DTV 数组、TLS 数据、Pthread 均在同一个 `libc.tls_size` 大小的内存块中
- 返回值指向的 Pthread 是完整初始化的 TCB（但部分字段尚未设置，需由 `init_tp` 完成）

**算法**:
1. 根据 `TLS_ABOVE_TP` 条件编译选择两种布局之一
2. 计算 DTV 数组位置（位于内存块的一端）
3. 计算 Pthread 的位置（位于内存块的另一端，按 `tls_align` 对齐）
4. `td.dtv = dtv` — DTV 指针写入 TCB
5. 遍历 `libc.tls_head` 链表，对每个模块执行 `ptr::copy_nonoverlapping` 拷贝初始化映像
6. 设置 `dtv[0] = libc.tls_cnt`

**Rust 实现要点**: 使用 `core::ptr::copy_nonoverlapping` 替代 C 的 `memcpy`，两者语义等价（源和目标不重叠）。

---

### `init_tls` — 主入口函数

```rust
// Rust 签名
pub(crate) fn init_tls(aux: *mut usize);
```

[Visibility]: Internal (`pub(crate)`) — rusl 运行时初始化函数，由 `crate::env::__libc_start_main` 调用。用户程序不可直接调用，POSIX/C 标准未定义

**C 对照**: `hidden void __init_tls(size_t *aux);` (`libc.h` 中声明为 `hidden`)

**意图**: 进程启动时的一次性 TLS 初始化。解析 ELF 辅助向量 (aux vector) 中的程序头信息，定位 PT_TLS 段，计算 TLS 模块布局，分配 TCB + TLS 内存，并完成主线程的 TLS 初始化。调用后 `pthread_self()` 可正常工作，`errno` / `locale` 等 TLS 变量可用。

**前置条件**:
- `aux` 指向辅助向量数组，其中 `aux[AT_PHDR]`、`aux[AT_PHNUM]`、`aux[AT_PHENT]` 已由内核/动态链接器填充
- 程序尚未初始化 TLS（`libc.tls_head == null_mut()`，`libc.tls_cnt == 0`）
- 这是进程启动后对 `init_tls` 的第一次调用（不可重入）
- `DEFAULT_STACKSIZE` 已被初始化为 `DEFAULT_STACK_SIZE`

**后置条件 (Case 1 — 正常路径)**:
1. **ELF 解析阶段**: `MAIN_TLS` 的 `image`、`len`、`size`、`align` 从 PT_TLS 程序头中提取；`libc.tls_head = MAIN_TLS.get()`；`libc.tls_cnt = 1`
2. **栈大小调整**: 若存在 `PT_GNU_STACK` 段且其 `p_memsz > DEFAULT_STACKSIZE`，则 `DEFAULT_STACKSIZE` 更新为 `p_memsz`（但不超过 `DEFAULT_STACK_MAX` = 8MB）
3. **TLS 布局计算**:
   - `main_tls.size` 向上对齐至 `main_tls.align`
   - `main_tls.offset` 计算：
     - TLS Below TP: `main_tls.offset = main_tls.size`
     - TLS Above TP: `main_tls.offset = GAP_ABOVE_TP`，再按 `main_tls.align` 对齐
   - `main_tls.align` = `max(main_tls.align, MIN_TLS_ALIGN)`
   - `libc.tls_align = main_tls.align`
   - `libc.tls_size = 2 * size_of::<usize>() + size_of::<Pthread>() + main_tls.offset + main_tls.size + main_tls.align`（再按 `MIN_TLS_ALIGN` 对齐）
     - 其中 `+ main_tls.offset` 仅在 `TLS_ABOVE_TP` 时参加计算（通过 `#[cfg(TLS_ABOVE_TP)]`）
4. **内存分配**:
   - 若 `libc.tls_size <= BUILTIN_TLS_SIZE`: 使用静态缓冲区 `BUILTIN_TLS`
   - 否则: `syscall::mmap(0, libc.tls_size, PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_PRIVATE, -1, 0)`
5. **TLS 拷贝**: 调用 `copy_tls(mem)`，返回初始化后的 `td: *mut Pthread`
6. **TCB 初始化**: 调用 `init_tp(td as *mut c_void)`
   - 若返回 < 0: **进程终止** (`core::intrinsics::abort()`)
   - 若返回 == 0: 主线程 TLS 完全就绪

**后置条件 (Case 2 — 无 PT_TLS 段)**:
- 若 ELF 遍历后 `tls_phdr == null()`（程序无 TLS 数据），则 `MAIN_TLS` 保持零初始化状态，但 `libc.tls_align`、`libc.tls_size`、`libc.tls_cnt`、`libc.tls_head` 均未被设置
- 后续 `copy_tls` 和 `init_tp` 仍被调用，但由于 `libc.tls_cnt == 0`，循环体不执行
- 实际效果：主线程的 TCB 被分配和初始化（含 thread pointer），但 TLS 变量不可用

**全局变量修改**:
- `libc.tls_head`, `libc.tls_size`, `libc.tls_align`, `libc.tls_cnt` — TLS 运行时状态
- `libc.can_do_threads` — 通过 `init_tp` 间接设置
- `DEFAULT_STACKSIZE` — 根据 `PT_GNU_STACK` 可能被更新
- `THREAD_LIST_LOCK` — 被 `init_tp` 作为 `SYS_set_tid_address` 的目标地址写入

**弱符号依赖**:
- `_DYNAMIC[]`: ELF 动态段地址，用于计算加载基址（当 PT_PHDR 的 `p_vaddr` 不可靠时，优先使用 `_DYNAMIC` 来计算 base）。若动态链接器未定义此符号（静态链接），其值为 0，此时回退到纯 PT_PHDR 方式计算 base。

**系统调用**:
- `SYS_mmap`: 仅当 `BUILTIN_TLS` 不足时调用
- 通过 `copy_tls` → 无系统调用
- 通过 `init_tp` → `SYS_set_thread_area` 或 `arch_prctl(ARCH_SET_FS)` + `SYS_set_tid_address`

**不变量**:
- **I1**: `libc.tls_align` 始终 >= `MIN_TLS_ALIGN`（保证 Pthread 对齐）
- **I2**: `libc.tls_size` 始终 >= `2 * size_of::<usize>() + size_of::<Pthread>() + main_tls.size + main_tls.align`（保证 DTV + TCB + TLS 数据 + 对齐填充的最小空间）
- **I3**: 初始化成功后，`pthread_self()` 可通过 `__get_tp()` 获得有效的 `*mut Pthread`
- **I4**: `MAIN_TLS` 的 `next` 指针为 `null_mut()`（主模块位于链表末尾）

**致命错误条件**:
- `mem` 分配成功但 `init_tp` 失败 → `core::intrinsics::abort()`
- `mmap` 返回的错误码（`(-4095..-1)`）会因后续解引用而触发 page fault，无需显式检查 — 与其在 Rust 中手动处理，不如依赖内核的惰性错误传递。但在 Rust 中应使用 `core::intrinsics::abort()` 作为显式终止路径，避免未定义行为。

---

## 全局变量

### `THREAD_LIST_LOCK`

```rust
// Rust 签名
pub(crate) static THREAD_LIST_LOCK: AtomicI32 = AtomicI32::new(0);
```

[Visibility]: Internal (`pub(crate)`) — rusl 线程系统内部全局锁，用户程序不可访问

**C 对照**: `volatile int __thread_list_lock;` (声明于 `pthread_impl.h` 为 `extern hidden`)

**定义位置**: `crate::internal::pthread_impl` 模块中定义，本模块通过 `init_tp` 使用其地址

**意图**: 保护全局线程链表 (`Pthread` 的 `prev`/`next` 双向链表) 的互斥锁。同时，其地址被用作 `SYS_set_tid_address` 的 `clear_child_tid` 参数——当线程退出时，内核零化此地址的值并通过 `futex(FUTEX_WAKE)` 唤醒等待者。

**双重用途**:
1. **作为锁值**: `AtomicI32`，由 `tl_lock()` / `tl_unlock()` 通过 CAS 操作实现互斥
2. **作为 TID 清除地址**: 其地址在 `init_tp` 中传递给 `SYS_set_tid_address`，作为线程退出时的通知机制

---

## 调用时序（进程启动流程）

```
_start (crt1.o / Scrt1.o)
  → __libc_start_main (crate::env::__libc_start_main)
    → __init_libc(envp, pn)     ← 解析 auxv，设置 libc.page_size, __hwcap, __sysinfo, __progname
    → init_tls(aux)              ← ★ 本模块定义的函数 ★
      → 解析 ELF PHDR, 计算 TLS 布局, 分配内存
        → copy_tls(mem)          ← 拷贝 TLS 初始数据 + 构造 DTV
        → init_tp(td)            ← 设置线程指针寄存器 + 初始化 TCB
    → __init_ssp(aux[AT_RANDOM]) ← 初始化栈保护 Canary
    → (...安全加固检查, 预初始化 fd...)
    → __libc_start_init()        ← 调用 .init / .init_array
    → main(argc, argv, envp)     ← 用户入口
```

---

## Rust 实现要点

1. **TLS 布局二选一**: 必须根据目标架构在编译期通过 `#[cfg(target_arch = "...")]` 选择 TLS Below TP 或 TLS Above TP 布局。这不能是运行时分支。
2. **BUILTIN_TLS 设计**: 使用 `UnsafeCell<BuiltinTls>` 作为静态缓冲区，其中 `BuiltinTls` 以 `#[repr(C)]` 布局确保 `Pthread` 字段的正确对齐。`MaybeUninit<Pthread>` 允许延迟初始化而无需在编译期构造完整的 Pthread。
3. **copy_tls 中的内存拷贝**: 使用 `core::ptr::copy_nonoverlapping` 代替 C 的 `memcpy`，两者语义等价（源和目标不重叠）且均为零开销。
4. **init_tp 中的系统调用**: 使用 rusl 内部的 `syscall` 模块进行系统调用，不使用任何外部 libc。
5. **ELF 解析**: 可复用 `goblin` crate（支持 `no_std`，需检查 no_std 兼容性）解析 ELF program headers，避免手动解析 `Phdr` 结构。也可直接在 rusl 内部实现最小化的 ELF 解析模块。
6. **致命错误处理**: `init_tp` 失败时使用 `core::intrinsics::abort()` 终止进程。不调用 `panic!`，因为在 TLS 初始化阶段，Rust 的 panic 基础设施可能尚未可用。
7. **no_std 约束**: 本模块完全不依赖 `std`，仅使用 `core::` 原语。`mmap` 等系统调用通过 rusl 内部的 `syscall` 模块直接封装。
8. **unsafe 范围控制**: 每个 `unsafe` 块应尽量小——仅包裹必须的裸指针操作或系统调用，避免大段 unsafe 代码。
9. **架构条件编译**: 使用 `#[cfg(...)]` 替代 C 的 `#ifdef`，确保编译器能检查所有架构分支的语法正确性。

---

/* Rely */
[RELY]
Rust Core 内建类型与宏:
  core::ptr::{null, null_mut, copy_nonoverlapping}                   // 内存操作
  core::mem::{size_of, offset_of}                                     // 编译期大小/偏移计算
  core::mem::MaybeUninit                                               // 延迟初始化
  core::cell::UnsafeCell                                              // 内部可变性
  core::ffi::c_void, core::ffi::c_int                                 // C FFI 类型
  core::intrinsics::abort                                              // 进程终止
  core::sync::atomic::AtomicI32                                        // 原子变量

rusl 内部模块:
  crate::internal::libc::{Libc, TlsModule, libc}                      // 依赖1: 全局运行时状态 + TLS 模块描述符
  crate::internal::libc::defsysinfo::__sysinfo                        // 依赖2: vDSO sysinfo 地址
  crate::internal::pthread_impl::{Pthread, DEFAULT_STACK_MAX, THREAD_LIST_LOCK}
                                                                      // 依赖3: 线程控制块 + 栈限制 + 线程列表锁
  crate::internal::pthread_impl::set_thread_area                      // 依赖4: 线程指针寄存器设置
  crate::internal::pthread_arch::{TLS_ABOVE_TP, GAP_ABOVE_TP, TP_OFFSET, DTP_OFFSET, TP_ADJ, __get_tp}
                                                                      // 依赖5: 架构相关 TLS 常量与辅助函数
  crate::syscall::{mmap, set_tid_address}                             // 依赖6: mmap 匿名映射 + set_tid_address 系统调用

外部 no_std crate (可选):
  goblin::elf64::program_header::{ProgramHeader, PT_PHDR, PT_DYNAMIC, PT_TLS, PT_GNU_STACK}
                                                                      // 依赖7(可选): ELF 程序头解析

[GUARANTEE]
Internal Interface (pub(crate)):
  fn init_tls(aux: *mut usize);                                       // 本模块保证: 主线程 TLS 初始化入口
  fn copy_tls(mem: *mut u8) -> *mut core::ffi::c_void;               // 本模块保证: TLS 初始数据拷贝 + DTV 构造
  fn init_tp(p: *mut core::ffi::c_void) -> core::ffi::c_int;         // 本模块保证: 线程指针初始化 + TCB 字段设置
