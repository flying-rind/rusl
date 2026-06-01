# vdso.c 规约

> 源文件: `/home/mangp/桌面/OS/musl/src/internal/vdso.c`
> 所属模块: musl 内部 vDSO 符号解析
> 复杂度层级: **Level 3** — 涉及复杂 ELF 解析、GNU hash 表遍历、版本依赖验证，需要显式描述系统算法

---

## 依赖图

```
__vdsosym ──> checkver (静态内部函数, 同文件)
         ──> count_syms_gnu (静态内部函数, 同文件)
         ──> strcmp (来自 <string.h>, C 标准库)
         ──> libc.auxv (来自 libc.h, 跨模块内部变量)
         ──> ELF 结构/常量 (来自 <elf.h>, <link.h>)
         ──> AT_SYSINFO_EHDR, DT_*, STT_*, STB_*, VER_FLG_BASE (来自 <elf.h>)

checkver ──> strcmp (来自 <string.h>)
         ──> Verdef/Verdaux 结构 (来自 <elf.h>)

count_syms_gnu ──> 仅使用 unsigned 算术和指针偏移, 无外部调用
```

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `libc.auxv` | `libc.h` | 外部模块内部变量，跳过（见 libc 规约） |
| ELF 结构体/常量 | `<elf.h>`, `<link.h>` | 系统标准头文件，跳过 |
| `strcmp` | `<string.h>` | C 标准库，跳过 |
| `Ehdr/Phdr/Sym/Verdef/Verdaux` | `<elf.h>` typedef | 系统标准头文件，跳过 |
| `DT_*`, `STT_*`, `STB_*`, `AT_SYSINFO_EHDR` | `<elf.h>` | 系统常量，跳过 |

---

## 背景：vDSO 机制 (Intent)

vDSO（virtual Dynamic Shared Object）是 Linux 内核提供的一种优化机制。内核将一小段只包含位置无关代码的 ELF 共享对象映射到每个进程的地址空间。用户态程序通过解析此 ELF 对象可以直接调用某些内核功能（如 `clock_gettime`、`gettimeofday` 等），而无需经过昂贵的内核态切换（syscall）。

### 内核接口

内核通过辅助向量（auxiliary vector, `auxv`）传递 vDSO 的地址：

```
auxv 条目: {AT_SYSINFO_EHDR, vdso_base_address}
```

其中 `vdso_base_address` 指向 vDSO ELF 的起始地址（ELF header）。

### musl 中的角色

`__vdsosym` 函数是 musl 查询 vDSO 符号地址的统一接口。调用者（如 `clock_gettime` 的实现）通过此函数获取 vDSO 中特定函数的地址，若 vDSO 不可用或符号不存在则返回 NULL，调用者回退到系统调用路径。

---

## 编译条件：VDSO_USEFUL

本文件全部内容受 `#ifdef VDSO_USEFUL` 保护。此宏在特定架构的构建配置中定义，仅在架构存在可利用的 vDSO 符号时才编译本文件。常见支持的架构包括 x86_64、aarch64、riscv64 等。

若 `VDSO_USEFUL` 未定义，整个文件编译为空——`__vdsosym` 的调用方需要通过其他方式处理。

---

## checkver (内部函数，静态)

### 签名

```c
static int checkver(Verdef *def, int vsym, const char *vername, char *strings);
```

### 可见性

**[Visibility]: Internal (不导出)** — `static` 函数，仅在 vdso.c 文件内可见。POSIX/C 标准未定义此符号。

### 意图 (Intent)

检查 ELF 符号的版本定义是否与请求的版本名称匹配。Linux vDSO 使用 ELF 版本化符号（versioned symbols），某些符号仅在特定版本下可用（如 `LINUX_2.6`）。此函数遍历 `Verdef` 链表，定位匹配的版本条目，然后比较版本名称。

### 前置条件

- `def` 指向有效的 `Verdef` 链表起始位置（来自 `DT_VERDEF` 动态条目）
- `vsym` 是来自 `versym` 数组的版本索引（高 1 位为隐藏位，需屏蔽）
- `vername` 是期望的版本名称（非 NULL，以 NUL 结尾）
- `strings` 指向 ELF 字符串表（DT_STRTAB）

### 后置条件

**Case 1: 版本匹配成功**

遍历 `Verdef` 链表找到与 `vsym & 0x7fff` 匹配的条目，且其 `Verdaux` 所引用的字符串等于 `vername`。
- 返回值: `1`（真）

**Case 2: 版本不匹配或版本定义不存在**

- 若遍历完链表后未找到匹配的版本索引（`vd_next == 0` 终止）：返回 `0`
- 若找到索引但版本名称不匹配：返回 `0`

### 遍历算法

```
while true:
  if (def->vd_flags & VER_FLG_BASE) == 0 AND (def->vd_ndx & 0x7fff) == (vsym & 0x7fff):
    break  // 找到匹配的版本索引
  if def->vd_next == 0:
    return 0  // 链表耗尽，未匹配
  def = (Verdef *)((char *)def + def->vd_next)  // 前进到下一个 Verdef
// 比较版本名称
aux = (Verdaux *)((char *)def + def->vd_aux)
return !strcmp(vername, strings + aux->vda_name)
```

**注意**: `VER_FLG_BASE` 标记的条目是基础版本定义（通常为无名称的全局符号），跳过这些条目。

---

## count_syms_gnu (内部函数，静态)

### 签名

```c
static size_t count_syms_gnu(uint32_t *gh);
```

### 可见性

**[Visibility]: Internal (不导出)** — `static` 函数，仅在 vdso.c 文件内可见。POSIX/C 标准未定义此符号。

### 意图 (Intent)

从 GNU 扩展 hash 表（`.gnu.hash`）中计算动态符号表的总条目数。GNU hash 表是 ELF 的一种可选 hash 格式（替代传统的 `DT_HASH`），采用 Bloom filter 加速查找。此函数解析 GNU hash 表结构以确定符号表的大小。

### GNU hash 表结构

```
struct gnu_hash_table {
    uint32_t nbuckets;      // gh[0]: hash 桶数量
    uint32_t symndx;        // gh[1]: 第一个可访问符号在 symtab 中的索引
    uint32_t maskwords;     // gh[2]: Bloom filter 的 mask 字数
    uint32_t shift2;        // gh[3]: Bloom filter 移位值
    // 后面是: maskwords 个 ElfW(Addr) Bloom filter words
    //          nbuckets 个 uint32_t hash buckets
    //          (nsym - symndx) 个 uint32_t hash values (chain)
};
```

### 前置条件

- `gh` 指向有效的 GNU hash 表起始地址（来自 `DT_GNU_HASH` 动态条目）

### 后置条件

- 返回值: 动态符号表的总条目数 `nsym`

### 系统算法

```
// gh[0]=nbuckets, gh[1]=symndx, gh[2]=maskwords
buckets = gh + 4 + (gh[2] * sizeof(size_t)/4)
// buckets 指向 nbuckets 个 uint32_t 桶索引数组

// 第一遍: 找最大的桶索引值
nsym = 0
for i in 0..nbuckets-1:
  if buckets[i] > nsym:
    nsym = buckets[i]

if nsym > 0:
  // hashval 指向桶之后的 hash chain 数组
  // 偏移 = nbuckets + (nsym - symndx)
  hashval = buckets + gh[0] + (nsym - gh[1])
  // 扫描 hash chain，找到最后一个条目（最低位为 1 标记链尾）
  do:
    nsym++
  while (!(*hashval++ & 1))

return nsym
```

**算法说明**:

1. **桶遍历**: GNU hash 表中每个 bucket 存储对应 hash 值链的起始符号索引。最大 bucket 值即包含有效符号的最小上界。
2. **链尾扫描**: hash chain 中每个 `uint32_t` 值的最低有效位（LSB）用作链终止标志。`(hashval & 1) == 1` 表示此条目是某个 hash 链的最后一个条目。算法从当前位置开始扫描，直到遇到链尾标记，从而确定符号表的确切大小。
3. **nsym 永远不会小于 symndx + 1**：因为至少有一个符号被导出。

---

## __vdsosym (内部函数)

### 签名

```c
void *__vdsosym(const char *vername, const char *name);
```

### 可见性

**[Visibility]: Internal (不导出)** — `hidden` 属性声明于 `syscall.h`。这是 musl 内部 vDSO 符号解析的对外接口，被 `__clock_gettime` 等时间相关系统调用的 fast-path 实现调用。POSIX/C 标准未定义此符号。

### 意图 (Intent)

从 Linux 内核提供的内核 vDSO 镜像中查找指定名称和版本的 ELF 动态符号的运行时地址。这是 musl 实现 "vsyscall" 快速路径的核心：找到 vDSO 中对应函数（如 `__vdso_clock_gettime`）的地址后，musl 直接调用它而非发起 syscall。

### 前置条件

- `vername` 和 `name` 均不为 NULL，指向有效的 NUL 结尾字符串
- 进程的 auxv 已由 `__init_libc` 初始化（`libc.auxv` 有效）
- 若 vDSO 不可用（内核未提供 `AT_SYSINFO_EHDR`），调用方应准备好回退路径
- 函数仅在编译时定义了 `VDSO_USEFUL` 的架构上可用

### 后置条件

**Case 1: 成功找到符号**

- 返回值: 符号的运行时虚拟地址（`base + syms[i].st_value`），可直接作为函数指针调用
- 返回的符号满足：类型为 `NOTYPE`/`OBJECT`/`FUNC`/`COMMON` 之一，且绑定为 `GLOBAL`/`WEAK`/`GNU_UNIQUE` 之一
- 若指定了 `versym` 且版本表存在，符号的版本名与 `vername` 匹配

**Case 2: 未找到符号或 vDSO 不可用**

- 返回值: `NULL`（0）
- 可能原因：
  - `AT_SYSINFO_EHDR` 不在 auxv 中
  - auxv 中该项的值为 0
  - vDSO ELF 缺少 `PT_DYNAMIC` 或 `PT_LOAD` 段
  - `DT_STRTAB` 或 `DT_SYMTAB` 缺失
  - 符号表中无匹配名称/类型的符号
  - 版本检查失败

### 系统算法 (System Algorithm)

#### 阶段 1: 定位 vDSO ELF

```
// 扫描 auxv 数组
for i=0; libc.auxv[i] != AT_SYSINFO_EHDR; i+=2:
  if libc.auxv[i] == 0 (AT_NULL):
    return 0  // auxv 已耗尽，无 vDSO

if libc.auxv[i+1] == 0:
  return 0  // AT_SYSINFO_EHDR 存在但地址为空

eh = (Ehdr *)libc.auxv[i+1]  // vDSO ELF header 地址
```

**auxv 结构**: `{type, value}` 键值对数组，以 `{AT_NULL, 0}` 终止。

#### 阶段 2: 解析 Program Headers

```
// 扫描 program headers 定位 PT_LOAD 和 PT_DYNAMIC
ph = (Phdr *)((char *)eh + eh->e_phoff)
base = -1
dynv = 0

for i=0; i < eh->e_phnum; i++:
  if ph->p_type == PT_LOAD:
    // 计算基址偏移: base = eh_addr + p_offset - p_vaddr
    // 后续通过 base + dynv[i+1] 解析虚拟地址
    base = (size_t)eh + ph->p_offset - ph->p_vaddr
  else if ph->p_type == PT_DYNAMIC:
    dynv = (void *)((char *)eh + ph->p_offset)

if !dynv || base == (size_t)-1:
  return 0  // 无法解析，vDSO 格式异常
```

**关键**: `base` 的计算公式允许从 ELF 文件中的偏移转换为运行时的虚拟地址。由于 vDSO 在内存中是以虚拟地址空间布局的（不是独立文件），必须通过 `p_offset - p_vaddr` 差值进行转换。

#### 阶段 3: 解析 Dynamic 段

```
// 扫描 DT_* 条目，提取符号/字符串/hash 表地址
strings = 0, syms = 0, hashtab = 0, ghashtab = 0, versym = 0, verdef = 0

for i=0; dynv[i] != DT_NULL; i+=2:
  p = (void *)(base + dynv[i+1])  // 虚拟地址 → 实际指针
  switch dynv[i]:
    case DT_STRTAB:  strings = p
    case DT_SYMTAB:  syms = p
    case DT_HASH:    hashtab = p
    case DT_GNU_HASH: ghashtab = p
    case DT_VERSYM:  versym = p
    case DT_VERDEF:  verdef = p

if !strings || !syms: return 0
```

#### 阶段 4: 计算符号表大小

```
nsym = 0
if hashtab:
  nsym = hashtab[1]  // ELF hash 表的 nchain 字段 = 符号总数
else if ghashtab:
  nsym = count_syms_gnu(ghashtab)  // 从 GNU hash 表推算
```

`DT_HASH` 结构：`[nbucket, nchain, buckets..., chains...]`，其中 `nchain`（`hashtab[1]`）等于符号表条目数。这是传统 ELF 的做法。

GNU hash 表不直接存储 nsym，必须通过 `count_syms_gnu` 推算。

#### 阶段 5: 线性扫描符号表

```
// 按 OK_TYPES 和 OK_BINDS 过滤，按名称/版本匹配
for i=0; i < nsym; i++:
  // 过滤 1: 类型检查
  if !(1 << ELF_ST_TYPE(syms[i].st_info) & OK_TYPES):
    continue
  
  // 过滤 2: 绑定检查
  if !(1 << ELF_ST_BIND(syms[i].st_info) & OK_BINDS):
    continue
  
  // 过滤 3: 节索引检查（undef 符号无 st_shndx）
  if !syms[i].st_shndx:
    continue
  
  // 过滤 4: 名称匹配
  if strcmp(name, strings + syms[i].st_name):
    continue
  
  // 过滤 5: 版本匹配（若存在版本表）
  if versym && !checkver(verdef, versym[i], vername, strings):
    continue
  
  // 全部匹配 — 返回运行时地址
  return (void *)(base + syms[i].st_value)

return 0  // 未找到
```

**过滤条件详解**:

```
OK_TYPES  = (1<<STT_NOTYPE) | (1<<STT_OBJECT) | (1<<STT_FUNC) | (1<<STT_COMMON)
OK_BINDS  = (1<<STB_GLOBAL)  | (1<<STB_WEAK)  | (1<<STB_GNU_UNIQUE)
```

`ELF_ST_TYPE(x) = x & 0xf`, `ELF_ST_BIND(x) = x >> 4` （实现为位操作 `syms[i].st_info&0xf` 和 `syms[i].st_info>>4`）

之所以接受 `STT_NOTYPE` 和 `STT_COMMON` 而不仅是 `STT_FUNC`，是因为某些 vDSO 符号（尤其是 `__kernel_*` 系列）可能被标记为 `NOTYPE`。

#### 阶段 6: 无条件版本检查

```
if !verdef: versym = 0  // 若版本定义表不存在，关闭版本检查
```

此逻辑确保：只有当 vDSO 同时提供了 `DT_VERDEF` 和 `DT_VERSYM` 时，才进行版本检查。若 `verdef` 缺失，即使 `versym` 存在也跳过版本过滤。

### 不变量

1. **vDSO 基址不变性**: 对于给定进程，vDSO 的加载地址在进程生命周期内不变。`__vdsosym` 的返回值在同一进程中始终有效（但不应跨 `fork` 后缓存，因为子进程可能接收不同的 vDSO 映射——实际上 Linux 内核在 `fork` 后保持相同映射）。

2. **故障安全**: 任何 ELF 解析异常（缺少段、格式错误）都导致返回 NULL，不会崩溃或返回无效指针。

3. **线性扫描保序**: 符号表按顺序扫描，返回第一个匹配项。对于重复符号（如 WEAK 覆盖），这保证了确定性行为。

### 性能特征

- **时间复杂度**: O(phnum + dynamic_entries + nsym)，其中 nsym 通常很小（vDSO 一般只导出少量符号，典型值 < 50）
- **空间复杂度**: O(1)，无需动态内存分配
- **典型调用模式**: 每个 vDSO 符号仅在首次使用时解析一次，结果被缓存（由调用方负责缓存，非本函数职责）

### Rust 实现注意事项

在 Rust `#![no_std]` 实现中：
- 需要直接读取 auxv（通过 `libc.auxv` 或等效机制）
- ELF 解析需要对原始内存进行 `unsafe` 读取，但可以将解析逻辑封装在安全接口内
- `strcmp` 可用 Rust 等效实现（字节级比较直到 NUL）
- 返回的地址应表示为 `Option<unsafe extern "C" fn(...)>` 或 `*const c_void`
- 在 `VDSO_USEFUL` 未定义的架构上，`__vdsosym` 应为空实现（返回 `null`）或编译时消除