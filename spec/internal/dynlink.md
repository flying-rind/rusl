# dynlink.h 规约

> **源文件**: `/home/mangp/桌面/OS/musl/src/internal/dynlink.h`
> **复杂度等级**: Level 3（高度优化设计 — 需要前置/后置条件 + 意图 + 显式系统算法）

---

## 依赖图

```
(外部) <features.h> ────────────┐
(外部) <elf.h> ────────────────┤
(外部) <stdint.h> ─────────────┤
(外部) <stddef.h> ─────────────┼──> dynlink.h ──> 使用者（动态链接器 / libc 初始化）
(外部) <stdarg.h> ─────────────┤
(架构) reloc.h ────────────────┘
                                    │
                                    ├── ELF 类型别名（Ehdr, Phdr, Sym）
                                    ├── 重定位辅助宏（R_TYPE, R_SYM, R_INFO, IS_RELATIVE）
                                    ├── 重定位类型枚举（REL_NONE .. REL_FUNCDESC_VAL）
                                    ├── FDPIC 数据结构（fdpic_loadseg, fdpic_loadmap）
                                    ├── 功能标志（DL_FDPIC, DL_NOMMU_SUPPORT, ...）
                                    ├── 常量（AUX_CNT=32, DYN_CNT=37）
                                    ├── 函数指针类型（stage2_func）
                                    ├── 导出函数（__dlsym, __dl_seterr, ...）
                                    └── 内部变量（__malloc_replaced, ...）
```

本文件是 musl 动态链接器的核心内部头文件，定义了 ELF 元数据处理、FDPIC 重定位、TLS 描述符解析、以及动态链接器与 libc 之间的内部通信接口。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `<features.h>` | musl 内部头文件 | **跨文件依赖** — 提供 `hidden`/`weak`/`weak_alias` 宏 |
| `<elf.h>` | glibc/musl 公共头文件 | 跳过 — 标准 ELF 结构定义 |
| `<stdint.h>`, `<stddef.h>`, `<stdarg.h>` | C 标准库 | 跳过 |
| `reloc.h` | musl 架构特定头文件 | **跨文件依赖** — 每个架构定义 `LDSO_ARCH` 和重定位类型常量 |

---

## 架构特定接口约定（reloc.h 应提供）

| 宏 | 含义 |
|----|------|
| `LDSO_ARCH` | 动态链接器架构名称字符串 |
| `REL_SYMBOLIC` | 架构的绝对 64 位重定位类型号 |
| `REL_OFFSET32` | PC 相对 32 位重定位 |
| `REL_GOT` | GOT 槽位重定位 |
| `REL_PLT` | PLT 跳转槽位重定位 |
| `REL_RELATIVE` | 相对重定位 |
| `REL_COPY` | 写时复制重定位 |
| `REL_DTPMOD` | TLS 模块 ID 重定位 |
| `REL_DTPOFF` | TLS 模块内偏移重定位 |
| `REL_TPOFF` | TLS 线程指针偏移重定位 |
| `REL_TLSDESC` | TLS 描述符重定位（可选） |
| `REL_FUNCDESC` | FDPIC 函数描述符重定位（可选） |
| `REL_FUNCDESC_VAL` | FDPIC 函数描述符值重定位（可选） |
| `REL_SYM_OR_REL` | 符号引用或相对重定位的歧义类型（可选） |
| `CRTJMP(pc,sp)` | 跳转到程序入口的汇编宏 |
| `GETFUNCSYM(fp, sym, got)` | 获取函数符号地址的汇编宏 |

---

## 符号规约

---

### 类型别名：`Ehdr`、`Phdr`、`Sym`

```c
#if UINTPTR_MAX == 0xffffffff
typedef Elf32_Ehdr Ehdr;
typedef Elf32_Phdr Phdr;
typedef Elf32_Sym Sym;
#else
typedef Elf64_Ehdr Ehdr;
typedef Elf64_Phdr Phdr;
typedef Elf64_Sym Sym;
#endif
```

[Visibility]: Internal — musl 内部类型别名。

#### 功能意图 (Intent)

根据系统字长（通过 `UINTPTR_MAX` 检测）选择正确的 ELF 结构体尺寸。避免在代码中使用 `#ifdef __LP64__` 等编译器宏，而是使用 `UINTPTR_MAX`（来自 `<stdint.h>`），使得在 ILP32-on-64 (x32) 等非标准 ABI 下选择正确。

#### 后置条件 (Postconditions)

- **POST-1**: `Ehdr` 为 `Elf32_Ehdr` 或 `Elf64_Ehdr`，匹配当前 ABI。
- **POST-2**: `Phdr` 同理匹配。
- **POST-3**: `Sym` 同理匹配。

---

### 重定位辅助宏：`R_TYPE`、`R_SYM`、`R_INFO`

```c
#if UINTPTR_MAX == 0xffffffff
#define R_TYPE(x) ((x)&255)
#define R_SYM(x) ((x)>>8)
#define R_INFO ELF32_R_INFO
#else
#define R_TYPE(x) ((x)&0x7fffffff)
#define R_SYM(x) ((x)>>32)
#define R_INFO ELF64_R_INFO
#endif
```

[Visibility]: Internal — musl 动态链接器内部宏。

#### 功能意图 (Intent)

从 ELF 重定位项 `r_info` 字段中提取重定位类型和符号表索引。32 位 ELF 使用 8 位类型码 + 24 位符号索引；64 位 ELF 使用 31 位类型码 + 32 位符号索引（注意 0x7fffffff 掩码的 31 位，最高位保留给其他用途）。

#### 不变量 (Invariants)

- **INV-1**: 在 32 位系统上，`R_TYPE(x)` 返回 0..255；在 64 位系统上，返回 0..0x7fffffff。
- **INV-2**: `R_INFO(sym, type)` 必须能由 `R_SYM` 和 `R_TYPE` 无损反向提取（对于 musl 支持的所有架构）。

---

### 重定位类型枚举

```c
enum {
    REL_NONE = 0,
    REL_SYMBOLIC = -100,
    REL_USYMBOLIC,
    REL_GOT,
    REL_PLT,
    REL_RELATIVE,
    REL_OFFSET,
    REL_OFFSET32,
    REL_COPY,
    REL_SYM_OR_REL,
    REL_DTPMOD,
    REL_DTPOFF,
    REL_TPOFF,
    REL_TPOFF_NEG,
    REL_TLSDESC,
    REL_FUNCDESC,
    REL_FUNCDESC_VAL,
};
```

[Visibility]: Internal — musl 动态链接器内部枚举。

#### 功能意图 (Intent)

为架构**不使用**的重定位类型提供"不可匹配"的默认值。这些枚举值被 `reloc.h` 中的架构特定 `#define` 覆盖为实际的 ELF 重定位类型号（如 x86_64 上 `REL_RELATIVE` 被重定义为 `R_X86_64_RELATIVE`）。

枚举从 `-100` 开始递减，确保这些默认值不会与任何合法的 ELF 重定位类型号（通常为小正整数）匹配。

#### 设计不变量

- **INV-1**: 若某个重定位类型在 `reloc.h` 中未被覆盖，使用此枚举中的负数值，在动态链接器中进行 switch-case 时将**永不匹配**任何实际的 ELF 重定位项，从而实现"该架构不支持此重定位"的语义。
- **INV-2**: 所有枚举值都是唯一的且递减排列，便于阅读和调试。

---

### FDPIC 数据结构

```c
struct fdpic_loadseg {
    uintptr_t addr, p_vaddr, p_memsz;
};

struct fdpic_loadmap {
    unsigned short version, nsegs;
    struct fdpic_loadseg segs[];
};

struct fdpic_dummy_loadmap {
    unsigned short version, nsegs;
    struct fdpic_loadseg segs[1];
};
```

[Visibility]: Internal — musl 动态链接器 FDPIC 支持所需。

#### 功能意图 (Intent)

**FDPIC**（Function Descriptor PIC）是 ELF 的一种变体，用于无 MMU 的嵌入式系统。FDPIC 程序使用**函数描述符**（包含函数地址和 GOT 指针）而非直接函数指针，并且每个共享库在其自己的地址空间中加载。

- `fdpic_loadseg`: 描述一个加载段——映射地址 `addr`、虚拟地址 `p_vaddr`、内存大小 `p_memsz`。
- `fdpic_loadmap`: 加载映射表——包含版本号、段数，以及变长的段数组。
- `fdpic_dummy_loadmap`: 固定大小（单段）的虚拟映射表，用于主程序（无需真实重定位时的占位符）。

#### 不变量 (Invariants)

- **INV-1**: `fdpic_loadmap.segs` 为灵活数组成员（FAM），实际大小由 `nsegs` 决定。
- **INV-2**: `fdpic_loadmap.version` 必须与内核期望的版本一致。
- **INV-3**: FDPIC 结构仅在 `DL_FDPIC` 编译选项启用时使用。

---

### 功能标志

```c
#ifndef FDPIC_CONSTDISP_FLAG
#define FDPIC_CONSTDISP_FLAG 0
#endif

#ifndef DL_FDPIC
#define DL_FDPIC 0
#endif

#ifndef DL_NOMMU_SUPPORT
#define DL_NOMMU_SUPPORT 0
#endif

#ifndef TLSDESC_BACKWARDS
#define TLSDESC_BACKWARDS 0
#endif

#ifndef NEED_MIPS_GOT_RELOCS
#define NEED_MIPS_GOT_RELOCS 0
#endif

#ifndef DT_DEBUG_INDIRECT
#define DT_DEBUG_INDIRECT 0
#endif

#ifndef DT_DEBUG_INDIRECT_REL
#define DT_DEBUG_INDIRECT_REL 0
#endif
```

[Visibility]: Internal — musl 编译期功能开关。

#### 功能意图 (Intent)

编译期功能开关，默认为 0（关闭）。由架构特定头文件在需要时覆盖为 1。这些标志控制动态链接器中的条件编译路径：

| 标志 | 含义 |
|------|------|
| `FDPIC_CONSTDISP_FLAG` | FDPIC 常量位移标志位掩码 |
| `DL_FDPIC` | 是否构建 FDPIC 动态链接器 |
| `DL_NOMMU_SUPPORT` | 是否支持无 MMU 系统的动态链接 |
| `TLSDESC_BACKWARDS` | TLS 描述符的 GOT 槽位是否以反向顺序排列 |
| `NEED_MIPS_GOT_RELOCS` | 是否需要 MIPS 特有的 GOT 重定位 |
| `DT_DEBUG_INDIRECT` | `.dynamic` 中 `DT_DEBUG` 是否为间接跳转 |
| `DT_DEBUG_INDIRECT_REL` | 是否需要为 `DT_DEBUG` 做重定位 |

---

### `IS_RELATIVE` 宏

```c
#if !DL_FDPIC
#define IS_RELATIVE(x,s) ( \
    (R_TYPE(x) == REL_RELATIVE) || \
    (R_TYPE(x) == REL_SYM_OR_REL && !R_SYM(x)) )
#else
#define IS_RELATIVE(x,s) ( ( \
    (R_TYPE(x) == REL_FUNCDESC_VAL) || \
    (R_TYPE(x) == REL_SYMBOLIC) ) \
    && (((s)[R_SYM(x)].st_info & 0xf) == STT_SECTION) )
#endif
```

[Visibility]: Internal — musl 动态链接器内部宏。

#### 功能意图 (Intent)

判断一个重定位项是否为"相对重定位"（即不需要符号解析，仅需基址偏移）。

1. **非 FDPIC 模式**: 若类型为 `REL_RELATIVE`，或类型为 `REL_SYM_OR_REL` 且符号索引为 0。
2. **FDPIC 模式**: 若类型为 `REL_FUNCDESC_VAL` 或 `REL_SYMBOLIC`，且目标符号类型为 `STT_SECTION`（节符号）。

#### 不变量 (Invariants)

- **INV-1**: `IS_RELATIVE` 结果为 1 的重定位项仅需要加载基址进行计算，不需要跨模块符号查找。
- **INV-2**: 在 `REL_SYM_OR_REL` 歧义情况下，`R_SYM(x) == 0` 是区分标志（ELF 标准规定符号索引 0 为 UNDEF 符号）。

---

### 常量

```c
#define AUX_CNT 32
#define DYN_CNT 37
```

[Visibility]: Internal — musl 动态链接器内部常量。

#### 功能意图 (Intent)

- `AUX_CNT = 32`: ELF 辅助向量（auxiliary vector）的最大条目数。辅助向量由内核在进程启动时传递，包含 AT_PHDR、AT_ENTRY 等信息。
- `DYN_CNT = 37`: `.dynamic` 段中 `DT_*` 标签的最大数量。

---

### `stage2_func` 类型

```c
typedef void (*stage2_func)(unsigned char *, size_t *);
```

[Visibility]: Internal — musl 动态链接器内部类型。

#### 功能意图 (Intent)

动态链接器第二阶段（stage 2）入口的函数指针类型。

- 第一个参数: 指向共享库加载基址的指针。
- 第二个参数: 指向运行时 DSO 计数的指针。

Stage 1 负责自举（bootstrap）重定位，Stage 2 负责加载所有依赖共享库并执行全部重定位。

---

### `__dlsym`

```c
hidden void *__dlsym(void *restrict, const char *restrict, void *restrict);
```

[Visibility]: Internal — musl 内部符号查找函数，但通过 `dlsym` 的公共实现间接暴露（`dlsym` 内部调用 `__dlsym`）。

#### 功能意图 (Intent)

在动态链接器或已加载共享库中查找符号。与公共 `dlsym` 相比，`__dlsym` 接受第三个参数（调用者的 `Dl_info` 或 DSO 上下文），支持从特定模块的视角查找符号。

#### 前置条件 (Preconditions)

- **PRE-1**: 第一个参数为有效的 DSO 句柄：`RTLD_DEFAULT`（全局搜索）、`RTLD_NEXT`（调用者之后搜索）、或 `dlopen` 返回的句柄。
- **PRE-2**: 第二个参数为以 null 结尾的符号名字符串。
- **PRE-3**: 第三个参数为调用者上下文（通常是调用模块的 DSO 指针），用于 `RTLD_NEXT` 和符号可见性计算。

#### 后置条件 (Postconditions)

- **Case 1 (成功)**:
  - **POST-1**: 返回符号的地址。
  - **POST-2**: 无错误状态。

- **Case 2 (符号未找到)**:
  - **POST-1**: 返回 `NULL`。
  - **POST-2**: 通过 `__dl_seterr` 设置错误消息。
  - **POST-3**: `dlerror()` 返回描述性错误字符串。

---

### `__dl_seterr`

```c
hidden void __dl_seterr(const char *, ...);
```

[Visibility]: Internal — musl 动态链接器内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

设置动态链接器错误消息（格式化字符串版本）。内部维护一个线程局部的错误缓冲区，供 `dlerror()` 读取。

#### 前置条件 (Preconditions)

- **PRE-1**: 格式字符串和可变参数遵循 `printf` 格式约定。
- **PRE-2**: 调用者必须是动态链接器内部代码路径。

#### 后置条件 (Postconditions)

- **POST-1**: 格式化后的错误消息存入线程局部的错误缓冲区。
- **POST-2**: 下一次 `dlerror()` 调用将返回该消息并清空缓冲区。

---

### `__dl_invalid_handle`

```c
hidden int __dl_invalid_handle(void *);
```

[Visibility]: Internal — musl 动态链接器内部函数。

#### 功能意图 (Intent)

检查给定的 DSO 句柄是否是有效的 `dlopen` 句柄。用于 `dlclose`、`dlsym` 等函数中的参数验证。

#### 前置条件 (Preconditions)

- **PRE-1**: 参数是用户通过 `dlopen` 获得的句柄或 `RTLD_DEFAULT`/`RTLD_NEXT` 特殊值。

#### 后置条件 (Postconditions)

- **POST-1**: 若句柄有效，返回 0。
- **POST-2**: 若句柄无效，返回非 0；调用者通常设置 `errno` 并返回错误。

---

### `__dl_vseterr`

```c
hidden void __dl_vseterr(const char *, va_list);
```

[Visibility]: Internal — musl 动态链接器内部函数。

#### 功能意图 (Intent)

`__dl_seterr` 的 `va_list` 版本，供内部函数在已有 `va_list` 时直接使用，避免 `va_start`/`va_end` 重复。

---

### `__tlsdesc_static` / `__tlsdesc_dynamic`

```c
hidden ptrdiff_t __tlsdesc_static(), __tlsdesc_dynamic();
```

[Visibility]: Internal — musl TLS 描述符解析函数。

#### 功能意图 (Intent)

TLS（Thread-Local Storage）描述符的两个解析函数：

- **`__tlsdesc_static`**: 解析静态 TLS 模型的访问——TLS 变量在主程序或启动时加载的共享库中定义，偏移量已知且固定。
- **`__tlsdesc_dynamic`**: 解析动态 TLS 模型的访问——TLS 变量在 `dlopen` 加载的共享库中定义，偏移量在运行时通过 `__tls_get_addr` 获取。

两个函数均无显式参数（使用 TLS 描述符的隐式参数寄存器/栈传递机制，取决于架构 ABI）。

#### 后置条件 (Postconditions)

- **POST-1**: 返回目标 TLS 变量相对于线程指针（`%fs`/`%gs`/`tp` 寄存器）的偏移量。
- **POST-2**: 对于 `__tlsdesc_static`，结果在加载时确定且不变。
- **POST-3**: 对于 `__tlsdesc_dynamic`，首次访问时可能触发 TLS 块分配。

---

### `__malloc_replaced` / `__aligned_alloc_replaced`

```c
hidden extern int __malloc_replaced;
hidden extern int __aligned_alloc_replaced;
```

[Visibility]: Internal — musl 内部全局标志变量。

#### 功能意图 (Intent)

指示用户程序是否通过 `LD_PRELOAD` 或静态链接替换了 musl 的 `malloc`/`aligned_alloc` 实现。动态链接器在加载时检测符号冲突并设置这些标志。若被替换，musl 内部的 `malloc` 调用路径需使用 `__libc_malloc` 或直接调用用户提供的实现。

#### 不变量 (Invariants)

- **INV-1**: `__malloc_replaced == 0` 表示 libc 内部可以使用直接路径调用 `malloc`。
- **INV-2**: `__malloc_replaced == 1` 表示用户替换了 `malloc`，内部代码必须通过 PLT 调用（避免符号版本冲突）。

---

### `__malloc_donate`

```c
hidden void __malloc_donate(char *, char *);
```

[Visibility]: Internal — musl 内部内存管理函数。

#### 功能意图 (Intent)

将一块连续内存区域"捐赠"给 musl 的 `malloc` 实现。用于动态链接器将不再需要的内存区域回收入 malloc 的内存池。

#### 前置条件 (Preconditions)

- **PRE-1**: `[start, end)` 是一块有效的、无别名连续内存区域。
- **PRE-2**: 该区域当前不被任何其他代码使用。

#### 后置条件 (Postconditions)

- **POST-1**: 该内存区域被纳入 `malloc` 内部空闲链表，可供后续 `malloc` 分配使用。

---

### `__malloc_allzerop`

```c
hidden int __malloc_allzerop(void *);
```

[Visibility]: Internal — musl 内部内存管理函数。

#### 功能意图 (Intent)

检查给定的内存块是否所有字节均为零。用于 `calloc` 的快速路径：当 `calloc` 从 `malloc` 获得的内存恰好全零时，可以跳过显式 `memset`。

#### 前置条件 (Preconditions)

- **PRE-1**: 参数指向有效的已分配内存块。

#### 后置条件 (Postconditions)

- **POST-1**: 若内存块所有字节为 0，返回非 0。
- **POST-2**: 若存在任何非零字节，返回 0。

---

## 全局不变量

- **GINV-1 (ELF 类型一致性)**: `Ehdr`、`Phdr`、`Sym` 的类型定义必须与当前 ABI 的字长一致，任何不匹配都会导致动态链接器解析 ELF 结构时崩溃。
- **GINV-2 (重定位类型兼容)**: 所有 `reloc.h` 中定义的重定位类型必须映射到枚举中的对应名称，且不产生常量冲突。
- **GINV-3 (TLS 安全)**: `__tlsdesc_static` 和 `__tlsdesc_dynamic` 必须在任何 TLS 访问之前被正确注册到 GCC/Clang 生成的 TLS 描述符调用点中。
- **GINV-4 (错误消息线程安全)**: `__dl_seterr` / `__dl_vseterr` 存储的错误消息是线程局部的，一个线程设置错误不影响另一个线程的 `dlerror()` 结果。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `reloc.h` | `arch/*/reloc.h` | 提供架构特定的重定位类型常量 |
| `__dlsym` 实现 | `src/ldso/dynlink.c` | 动态链接器核心实现 |
| `__dl_seterr` / `__dl_vseterr` 实现 | `src/ldso/dynlink.c` | 错误消息管理 |
| `__dl_invalid_handle` 实现 | `src/ldso/dynlink.c` | 句柄验证 |
| `__tlsdesc_static` / `__tlsdesc_dynamic` 实现 | `src/ldso/` 或架构特定汇编 | TLS 解析 |
| `__malloc_replaced` / `__aligned_alloc_replaced` | `src/ldso/dynlink.c` | 符号替换检测 |
| `__malloc_donate` / `__malloc_allzerop` 实现 | `src/malloc/` | malloc 内部接口 |

---

## Rust 实现提示 (`#![no_std]`)

在 `rusl` 中实现此模块时，由于 Rust 的标准链接模型与 C 的动态链接器不同，通常 `dynlink.h` 的功能分为两个部分：

1. **ELF 解析部分**（`Ehdr`、`Phdr`、`Sym`）:
   - 使用 `goblin` crate（`#![no_std]` 兼容）的 `elf` 模块代替 `<elf.h>`。
   - 或手工定义与 ELF 结构匹配的 `#[repr(C)]` 结构体。

2. **动态链接器运行时**:
   - musl 的动态链接器 (`ld-musl-*.so.1`) 是独立于 `rusl` 的组件。
   - 若 `rusl` 仅作为静态库链接，可能不需要实现动态链接器本身。
   - 若需要 `dlopen`/`dlsym` 功能，可使用内联汇编 + `mmap` + ELF 解析实现。
   - `__malloc_replaced` 等 musl 特有的符号替换机制在纯 Rust 环境中通常不需要（Rust 无 LD_PRELOAD 的等效机制）。