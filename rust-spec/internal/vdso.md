# vdso 规约 (Rust)

> **来源文件**: `musl/src/internal/vdso.c`
> **目标模块**: `rusl/src/internal/vdso.rs`
> **复杂度层级**: Level 3 — 涉及复杂 ELF 解析、GNU hash 表遍历、版本依赖验证

---

## 概述

`vdso` 模块实现了从 Linux vDSO（virtual Dynamic Shared Object）中查找符号地址的功能。vDSO 是内核映射到用户态的一段 ELF 共享对象，允许用户态直接调用某些内核功能（如 `clock_gettime`），无需经过系统调用。

`__vdsosym` 是 musl/rusl 查询 vDSO 符号地址的统一接口。

---

## [RELY]

```
Predefined Structures/Functions:
  // libc.auxv — 辅助向量（进程启动时初始化）
  static libc_auxv: &[[usize; 2]];              // 依赖: auxv 辅助向量数组（{type, value} 键值对）
  
  // ELF 结构体（手动定义或引用 elf crate）
  #[repr(C)]
  struct Ehdr { elf_header_fields... };          // ELF header
  #[repr(C)]
  struct Phdr { program_header_fields... };      // Program header
  #[repr(C)]
  struct Dyn { d_tag: isize, d_val: usize };     // Dynamic 条目
  #[repr(C)]
  struct Sym { st_name: u32, st_info: u8, st_other: u8, st_shndx: u16, st_value: usize, st_size: usize }; // 符号表条目
  #[repr(C)]
  struct Verdef { vd_version: u16, vd_flags: u16, vd_ndx: u16, vd_cnt: u16, vd_hash: u32, vd_aux: u32, vd_next: u32 }; // 版本定义
  #[repr(C)]
  struct Verdaux { vda_name: u32, vda_next: u32 }; // 版本定义辅助条目
  
  // ELF 常量
  const AT_SYSINFO_EHDR: usize = 33;
  const AT_NULL: usize = 0;
  const PT_LOAD: u32 = 1;
  const PT_DYNAMIC: u32 = 2;
  const DT_NULL: isize = 0;
  const DT_STRTAB: isize = 5;
  const DT_SYMTAB: isize = 6;
  const DT_HASH: isize = 4;
  const DT_GNU_HASH: isize = 0x6ffffef5;
  const DT_VERSYM: isize = 0x6ffffff0;
  const DT_VERDEF: isize = 0x6ffffffc;
  const STT_NOTYPE: u8 = 0;
  const STT_OBJECT: u8 = 1;
  const STT_FUNC: u8 = 2;
  const STT_COMMON: u8 = 5;
  const STB_GLOBAL: u8 = 1;
  const STB_WEAK: u8 = 2;
  const STB_GNU_UNIQUE: u8 = 10;
  const VER_FLG_BASE: u16 = 0x1;
  // OK_TYPES 和 OK_BINDS 位掩码
  const OK_TYPES: u32  = (1 << STT_NOTYPE) | (1 << STT_OBJECT) | (1 << STT_FUNC) | (1 << STT_COMMON);
  const OK_BINDS: u32  = (1 << STB_GLOBAL) | (1 << STB_WEAK) | (1 << STB_GNU_UNIQUE);
```

## [GUARANTEE]

```
Exported Interface:
  fn __vdsosym(vername: *const c_char, name: *const c_char) -> *mut c_void;
                                  // [Visibility]: Internal (不导出)
                                  // musl 内部 vDSO 符号查找接口
```

---

## 编译条件: VDSO_USEFUL

本模块全部内容受条件编译控制，仅在支持 vDSO 的架构上编译：

```rust
// 仅在定义了 VDSO_USEFUL 的架构上编译
#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64",
    // ... 其他支持 vDSO 的架构
))]
```

若 `VDSO_USEFUL` 条件不成立，`__vdsosym` 实现为空（始终返回 `null_mut()`）。

---

## 内部函数

### `checkver` — 版本定义匹配检查

```rust
/// 检查 ELF 符号的版本定义是否与请求的版本名称匹配
///
/// 遍历 Verdef 链表，定位匹配的版本条目，比较版本名称。
///
/// # Returns
/// - `true`: 版本匹配成功
/// - `false`: 版本不匹配或版本定义不存在
fn checkver(
    def: *const Verdef,
    vsym: u16,
    vername: *const c_char,
    strings: *const c_char,
) -> bool;
```

`[Visibility]: Internal — 模块私有函数`

**遍历算法**:
```
while true:
  if (def.vd_flags & VER_FLG_BASE) == 0 AND (def.vd_ndx & 0x7fff) == (vsym & 0x7fff):
    break  // 找到匹配的版本索引
  if def.vd_next == 0:
    return false  // 链表耗尽，未匹配
  def = def + def.vd_next  // 前进到下一个 Verdef
// 比较版本名称
aux = def + def.vd_aux
return strcmp(vername, strings + aux.vda_name) == 0
```

---

### `count_syms_gnu` — GNU hash 表符号计数

```rust
/// 从 GNU 扩展 hash 表（.gnu.hash）中计算动态符号表的总条目数
///
/// GNU hash 表结构:
///   gh[0] = nbuckets (hash 桶数量)
///   gh[1] = symndx (第一个可访问符号索引)
///   gh[2] = maskwords (Bloom filter mask 字数, 32 位下为 sizeof(usize)/4)
///   gh[3] = shift2 (Bloom filter 移位值)
///   然后: maskwords 个 usize 的 Bloom filter
///        nbuckets 个 u32 的 hash buckets
///  最后是 hash chain
fn count_syms_gnu(gh: *const u32) -> usize;
```

`[Visibility]: Internal — 模块私有函数`

**系统算法**:
```
buckets = gh + 4 + (gh[2] * (core::mem::size_of::<usize>() / 4))

// 第一遍: 找最大的桶索引值
nsym = 0
for i in 0..gh[0]-1:
  if buckets[i] > nsym:
    nsym = buckets[i]

if nsym > 0:
  // hashval 指向桶之后的 hash chain
  hashval = buckets + gh[0] + (nsym - gh[1])
  // 扫描 hash chain，找到链尾标记（最低位为 1）
  do:
    nsym += 1
  while (*hashval & 1) == 0
    hashval += 1

return nsym
```

---

## 对外函数

### `__vdsosym` — vDSO 符号查找

```rust
/// 从 Linux 内核提供的 vDSO 镜像中查找指定名称和版本的 ELF 动态符号地址
///
/// # Safety
///
/// `vername` 和 `name` 均不为空，指向有效的 NUL 结尾字符串。
/// 进程的 auxv 必须已初始化（libc_auxv 有效）。
///
/// # Returns
///
/// - 成功: 符号的运行时虚拟地址，可直接作为函数指针调用
/// - 失败: `null_mut()` — vDSO 不可用、符号不存在、或任何解析异常
#[no_mangle]
pub unsafe extern "C" fn __vdsosym(
    vername: *const c_char,
    name: *const c_char,
) -> *mut c_void;
```

`[Visibility]: Internal — 被 syscall.h 声明为 hidden`

---

### 意图 (Intent)

从 Linux 内核提供的 vDSO 镜像中查找指定名称和版本的 ELF 动态符号的运行时地址。这是 rusl 实现 vsyscall 快速路径的核心。

---

### 前置条件

- `vername` 和 `name` 均不为 NULL，指向有效的 NUL 结尾字符串
- 进程的 auxv 已由启动代码初始化
- 若 vDSO 不可用，调用方应准备好回退路径
- 函数仅在编译时条件满足的架构上可用

---

### 后置条件

**Case 1: 成功找到符号**
- 返回值: 符号的运行时虚拟地址（`base + sym.st_value`）
- 返回的符号满足：类型为 NOTYPE/OBJECT/FUNC/COMMON 之一，绑定为 GLOBAL/WEAK/GNU_UNIQUE 之一
- 若指定了版本表且版本表存在，符号的版本名与 `vername` 匹配

**Case 2: 未找到符号或 vDSO 不可用**
- 返回值: `null_mut()`
- 可能原因：`AT_SYSINFO_EHDR` 不在 auxv 中、vDSO ELF 格式异常、符号表缺失、名称/版本不匹配

---

### 系统算法 (System Algorithm)

#### 阶段 1: 定位 vDSO ELF

```
扫描 auxv 数组，查找 AT_SYSINFO_EHDR 条目
若未找到或值为 0 → return null_mut()
eh = auxv_value  // Ehdr 指针
```

#### 阶段 2: 解析 Program Headers

```
扫描 program headers，定位 PT_LOAD 和 PT_DYNAMIC:
  base = (eh as usize) + ph.p_offset - ph.p_vaddr
  dynv = (eh as usize) + ph.p_offset  // 指向动态段

若 dynv 为空或 base 为初始值(-1) → return null_mut()
```

#### 阶段 3: 解析 Dynamic 段

```
扫描 DT_* 条目，提取:
  strings  (DT_STRTAB)
  syms     (DT_SYMTAB)
  hashtab  (DT_HASH)    或
  ghashtab (DT_GNU_HASH)
  versym   (DT_VERSYM)
  verdef   (DT_VERDEF)

若 strings 或 syms 为空 → return null_mut()
```

#### 阶段 4: 计算符号表大小

```
nsym = 0
若 hashtab 存在 → nsym = hashtab[1]  (nchain)
否则若 ghashtab 存在 → nsym = count_syms_gnu(ghashtab)
```

#### 阶段 5: 线性扫描符号表

```
for i in 0..nsym:
  过滤 1: 类型检查 → ELF_ST_TYPE(sym.st_info) 在 OK_TYPES 中
  过滤 2: 绑定检查 → ELF_ST_BIND(sym.st_info) 在 OK_BINDS 中
  过滤 3: 节索引检查 → sym.st_shndx != 0
  过滤 4: 名称匹配 → strcmp(name, strings + sym.st_name)
  过滤 5: 版本匹配 → 若 versym 和 verdef 存在 → checkver(...)
  全部匹配 → return (base + sym.st_value) as *mut c_void

return null_mut()
```

#### 阶段 6: 无条件版本检查

```
若 verdef 为空 → versym = null  // 关闭版本检查
```

---

### 不变量

1. **vDSO 基址不变性**: 对于给定进程，vDSO 的加载地址在进程生命周期内不变。
2. **故障安全**: 任何 ELF 解析异常都导致返回 `null_mut()`，不会崩溃或返回无效指针。
3. **线性扫描保序**: 符号表按顺序扫描，返回第一个匹配项。

---

### 性能特征

- **时间复杂度**: O(phnum + dynamic_entries + nsym)，nsym 通常 < 50
- **空间复杂度**: O(1)，无需动态内存分配
- **典型调用模式**: 每个 vDSO 符号仅在首次使用时解析一次，结果由调用方缓存

---

### Rust 实现注意事项

在 Rust `#![no_std]` 实现中：
- 需要直接读取 auxv（通过全局静态数组或等效机制）
- ELF 解析需要对原始内存进行 `unsafe` 读取——将每个阶段封装在独立的内部函数中，最小化 unsafe 范围
- `strcmp` 用 Rust 等效实现（字节级比较直到 NUL）
- 返回的地址表示为 `*mut c_void`
- 在 `VDSO_USEFUL` 未定义的架构上，`__vdsosym` 应为空实现或编译时消除
- 使用 `#[cfg(...)]` 条件编译替代 C 的 `#ifdef VDSO_USEFUL`