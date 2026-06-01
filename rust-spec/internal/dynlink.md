# dynlink 模块规约 (Rust)

> **源 C spec**: `/home/mangp/桌面/OS/musl/src/internal/spec/dynlink.md`
> **复杂度等级**: Level 3（高度复杂 — ELF 解析 + 动态重定位 + TLS 描述符）

---

## 依赖图

```
core::ptr / core::mem ──> dynlink 模块
                              │
                              ├── ELF 类型别名（Ehdr, Phdr, Sym 等）
                              ├── 重定位辅助宏/函数（r_type, r_sym, r_info, is_relative）
                              ├── 重定位类型枚举（RelType）
                              ├── FDPIC 数据结构（fdpic_loadseg, fdpic_loadmap）
                              ├── 功能标志常量
                              ├── 顶层常量（AUX_CNT, DYN_CNT）
                              ├── Stage2Func 类型别名
                              ├── 导出符号（__dlsym, __dl_seterr, ...）
                              └── 内部变量（__malloc_replaced, ...）
```

本模块是 rusl 动态链接器的核心内部模块，定义了 ELF 元数据处理、FDPIC 重定位、TLS 描述符解析、以及动态链接器与 libc 之间的内部通信接口。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::ffi::c_*` | Rust core 库 | C 兼容类型 |
| `crate::internal::elf` | rusl 内部 ELF 解析模块 | 提供 `#[repr(C)]` ELF 结构体定义 |

### ELF 结构体来源

rusl 中不依赖外部 `<elf.h>`，而是自行定义 `#[repr(C)]` 的 ELF 结构体。这些结构体定义在 `crate::internal::elf` 模块中，按 32 位 / 64 位分别定义：

```rust
// 在 crate::internal::elf 模块中定义
pub mod elf32 { ... }  // Elf32_Ehdr, Elf32_Phdr, Elf32_Sym, ...
pub mod elf64 { ... }  // Elf64_Ehdr, Elf64_Phdr, Elf64_Sym, ...
```

---

## 架构特定接口约定

每个目标架构通过条件编译模块提供以下常量：

| 常量 | 类型 | 含义 |
|------|------|------|
| `LDSO_ARCH` | `&'static str` | 动态链接器架构名称字符串 |
| `REL_SYMBOLIC` | `u32` | 架构的绝对 64 位重定位类型号 |
| `REL_OFFSET32` | `u32` | PC 相对 32 位重定位 |
| `REL_GOT` | `u32` | GOT 槽位重定位 |
| `REL_PLT` | `u32` | PLT 跳转槽位重定位 |
| `REL_RELATIVE` | `u32` | 相对重定位 |
| `REL_COPY` | `u32` | 写时复制重定位 |
| `REL_DTPMOD` | `u32` | TLS 模块 ID 重定位 |
| `REL_DTPOFF` | `u32` | TLS 模块内偏移重定位 |
| `REL_TPOFF` | `u32` | TLS 线程指针偏移重定位 |
| `REL_TLSDESC` | `u32` | TLS 描述符重定位（可选） |
| `REL_FUNCDESC` | `u32` | FDPIC 函数描述符重定位（可选） |
| `REL_FUNCDESC_VAL` | `u32` | FDPIC 函数描述符值重定位（可选） |
| `REL_SYM_OR_REL` | `u32` | 符号引用或相对重定位的歧义类型（可选） |

示例：x86_64 架构模块

```rust
// src/internal/arch/x86_64/mod.rs
pub(crate) const LDSO_ARCH: &str = "x86_64";
pub(crate) const REL_SYMBOLIC: u32 = 0;    // x86_64 无独立符号重定位
pub(crate) const REL_RELATIVE: u32 = 8;     // R_X86_64_RELATIVE
pub(crate) const REL_GOT: u32 = 0;          // x86_64 无此类型
pub(crate) const REL_PLT: u32 = 0;          // x86_64 无此类型
pub(crate) const REL_COPY: u32 = 5;         // R_X86_64_COPY
pub(crate) const REL_TPOFF: u32 = 18;       // R_X86_64_TPOFF64
pub(crate) const REL_DTPMOD: u32 = 16;      // R_X86_64_DTPMOD64
pub(crate) const REL_DTPOFF: u32 = 17;      // R_X86_64_DTPOFF64
pub(crate) const REL_TLSDESC: u32 = 36;     // R_X86_64_TLSDESC
pub(crate) const REL_FUNCDESC: u32 = 0;     // 不支持 FDPIC
pub(crate) const REL_FUNCDESC_VAL: u32 = 0; // 不支持 FDPIC
pub(crate) const REL_SYM_OR_REL: u32 = 0;   // x86_64 无歧义类型
pub(crate) const REL_OFFSET32: u32 = 0;     // x86_64 无此类型
```

---

## 符号规约

---

### ELF 类型别名

```rust
// Rust 声明 — 根据 usize 字长自动选择 32/64 位 ELF 类型
#[cfg(target_pointer_width = "32")]
pub(crate) use crate::internal::elf::elf32::{
    Ehdr as ElfEhdr,
    Phdr as ElfPhdr,
    Sym as ElfSym,
};

#[cfg(target_pointer_width = "64")]
pub(crate) use crate::internal::elf::elf64::{
    Ehdr as ElfEhdr,
    Phdr as ElfPhdr,
    Sym as ElfSym,
};
```

[Visibility]: Internal — rusl 内部类型别名。

#### 功能意图 (Intent)

根据目标平台字长（通过 `target_pointer_width` 编译期属性）选择正确的 ELF 结构体尺寸。与 C 版本使用 `UINTPTR_MAX` 运行时宏的方式不同，Rust 在编译期即可确定类型。

#### 后置条件 (Postconditions)

- **POST-1**: `ElfEhdr` 为 `elf32::Ehdr` 或 `elf64::Ehdr`，匹配目标 ABI。
- **POST-2**: `ElfPhdr` 同理匹配。
- **POST-3**: `ElfSym` 同理匹配。

---

### 重定位辅助函数

```rust
// Rust 声明 — 从 ELF 重定位项 r_info 字段中提取信息
#[cfg(target_pointer_width = "32")]
pub(crate) fn r_type(x: u32) -> u32 { x & 0xFF }

#[cfg(target_pointer_width = "64")]
pub(crate) fn r_type(x: u64) -> u32 { (x & 0x7FFF_FFFF) as u32 }

#[cfg(target_pointer_width = "32")]
pub(crate) fn r_sym(x: u32) -> u32 { x >> 8 }

#[cfg(target_pointer_width = "64")]
pub(crate) fn r_sym(x: u64) -> u32 { (x >> 32) as u32 }

// r_info: 组合符号索引和重定位类型为 r_info 值
// 在 32 位：r_info(sym, type) 等价于 ELF32_R_INFO
// 在 64 位：r_info(sym, type) 等价于 ELF64_R_INFO
#[cfg(target_pointer_width = "32")]
pub(crate) fn r_info(sym: u32, ty: u32) -> u32 { (sym << 8) | (ty & 0xFF) }

#[cfg(target_pointer_width = "64")]
pub(crate) fn r_info(sym: u32, ty: u32) -> u64 { ((sym as u64) << 32) | (ty as u64 & 0x7FFF_FFFF) }
```

[Visibility]: Internal — rusl 动态链接器内部辅助函数。

#### 功能意图 (Intent)

从 ELF 重定位项 `r_info` 字段中提取重定位类型和符号表索引。32 位 ELF 使用 8 位类型码 + 24 位符号索引；64 位 ELF 使用 31 位类型码 + 32 位符号索引。

#### 不变量 (Invariants)

- **INV-1**: 在 32 位目标上，`r_type(x)` 返回 0..255；在 64 位目标上，返回 0..0x7FFF_FFFF。
- **INV-2**: `r_info(r_sym(x), r_type(x)) == x` 对于合法的 `x` 成立。

---

### 重定位类型枚举

```rust
// Rust 声明 — 重定位类型默认值枚举（当架构未定义对应常量时使用）
pub(crate) struct RelType;

impl RelType {
    pub(crate) const NONE: u32 = 0;
    pub(crate) const SYMBOLIC: u32 = u32::MAX - 99; // 等效于 C 的 "= -100"
    pub(crate) const USYMBOLIC: u32 = u32::MAX - 98;
    pub(crate) const GOT: u32 = u32::MAX - 97;
    pub(crate) const PLT: u32 = u32::MAX - 96;
    pub(crate) const RELATIVE: u32 = u32::MAX - 95;
    pub(crate) const OFFSET: u32 = u32::MAX - 94;
    pub(crate) const OFFSET32: u32 = u32::MAX - 93;
    pub(crate) const COPY: u32 = u32::MAX - 92;
    pub(crate) const SYM_OR_REL: u32 = u32::MAX - 91;
    pub(crate) const DTPMOD: u32 = u32::MAX - 90;
    pub(crate) const DTPOFF: u32 = u32::MAX - 89;
    pub(crate) const TPOFF: u32 = u32::MAX - 88;
    pub(crate) const TPOFF_NEG: u32 = u32::MAX - 87;
    pub(crate) const TLSDESC: u32 = u32::MAX - 86;
    pub(crate) const FUNCDESC: u32 = u32::MAX - 85;
    pub(crate) const FUNCDESC_VAL: u32 = u32::MAX - 84;
}
```

> 注意：当架构模块（如 `arch/x86_64`）定义了特定常量后，应使用 `use arch::REL_RELATIVE as RELATIVE;` 覆盖这些默认值。这些默认值从 `u32::MAX - 99`（0xFFFF FF9D）开始递减，**永不匹配**任何合法的 ELF 重定位类型号（通常为小正整数）。

[Visibility]: Internal — rusl 动态链接器内部枚举。

#### 功能意图 (Intent)

为架构**不使用**的重定位类型提供"不可匹配"的默认值。这些值的设计原理与 C 版本相同——使用极大的正数值（等效于从 -100 递减），确保在动态链接器中进行 `match` 或查找时**永不匹配**任何实际的 ELF 重定位项。

#### 设计不变量

- **INV-1**: 若某个重定位类型在架构模块中未被覆盖，使用此枚举中的值，任何重定位查找将**永不匹配**，自然实现"该架构不支持此重定位"的语义。
- **INV-2**: 所有枚举值唯一且可读。

---

### `is_relative` 函数

```rust
// Rust 声明 — 判断重定位项是否为"相对重定位"
#[cfg(not(feature = "fdpic"))]
pub(crate) fn is_relative(rel_type: u32, sym_idx: u32) -> bool {
    rel_type == arch::REL_RELATIVE
        || (rel_type == arch::REL_SYM_OR_REL && sym_idx == 0)
}

#[cfg(feature = "fdpic")]
pub(crate) fn is_relative(rel_type: u32, sym: &ElfSym) -> bool {
    (rel_type == arch::REL_FUNCDESC_VAL || rel_type == arch::REL_SYMBOLIC)
        && (sym.st_info & 0xF) == STT_SECTION
}
```

[Visibility]: Internal — rusl 动态链接器内部函数。

#### 功能意图 (Intent)

判断一个重定位项是否为"相对重定位"（即不需要符号解析，仅需基址偏移）。

1. **非 FDPIC 模式**: 若类型为 `REL_RELATIVE`，或类型为 `REL_SYM_OR_REL` 且符号索引为 0（UNDEF 符号）。
2. **FDPIC 模式**: 若类型为 `REL_FUNCDESC_VAL` 或 `REL_SYMBOLIC`，且目标符号类型为 `STT_SECTION`。

#### 不变量 (Invariants)

- **INV-1**: `is_relative` 返回 `true` 的重定位项仅需要加载基址进行计算，不需要跨模块符号查找。
- **INV-2**: 在 `REL_SYM_OR_REL` 歧义情况下，符号索引 0（UNDEF 符号）是区分标志。

---

### FDPIC 数据结构

```rust
// Rust 声明 — FDPIC 段描述符和加载映射表
#[repr(C)]
pub(crate) struct FdpicLoadseg {
    pub addr: usize,
    pub p_vaddr: usize,
    pub p_memsz: usize,
}

#[repr(C)]
pub(crate) struct FdpicLoadmap {
    pub version: u16,
    pub nsegs: u16,
    // segs: 灵活数组成员，Rust 中通过 DST 或裸指针模拟
    // 在访问时: unsafe { (*ptr).segs_ptr().add(i) }
}

impl FdpicLoadmap {
    /// 获取段数组的指针
    pub(crate) unsafe fn segs_ptr(&self) -> *const FdpicLoadseg {
        // 紧跟在结构体尾部
        (self as *const Self).add(1) as *const FdpicLoadseg
    }

    /// 版本兼容性检查
    pub(crate) fn version_ok(&self) -> bool {
        self.version == 1
    }
}

#[repr(C)]
pub(crate) struct FdpicDummyLoadmap {
    pub version: u16,
    pub nsegs: u16,
    pub segs: [FdpicLoadseg; 1],
}
```

[Visibility]: Internal — rusl FDPIC 支持所需（仅在启用 `fdpic` feature 时编译）。

#### 功能意图 (Intent)

FDPIC 是 ELF 的一种变体，用于无 MMU 的嵌入式系统。仅在 `cfg(feature = "fdpic")` 启用时编译。大多数桌面/服务器平台（x86_64、aarch64）不需要此功能。

- `FdpicLoadseg`: 描述一个加载段——映射地址 `addr`、虚拟地址 `p_vaddr`、内存大小 `p_memsz`。
- `FdpicLoadmap`: 加载映射表——包含版本号、段数，以及变长的段数组。
- `FdpicDummyLoadmap`: 固定大小的单段映射表占位符。

#### 不变量 (Invariants)

- **INV-1**: `segs` 为灵活数组成员，实际大小由 `nsegs` 决定。
- **INV-2**: `version` 必须与内核期望的版本一致（当前为 1）。

---

### 功能标志常量

```rust
// Rust 声明 — 编译期功能开关
/// 是否构建 FDPIC 动态链接器
#[cfg(feature = "fdpic")]
pub(crate) const DL_FDPIC: bool = true;
#[cfg(not(feature = "fdpic"))]
pub(crate) const DL_FDPIC: bool = false;

/// 是否支持无 MMU 系统的动态链接
pub(crate) const DL_NOMMU_SUPPORT: bool = false;

/// TLS 描述符 GOT 槽位是否反向排列
pub(crate) const TLSDESC_BACKWARDS: bool = false;

/// 是否需要 MIPS 特有的 GOT 重定位
pub(crate) const NEED_MIPS_GOT_RELOCS: bool = false;

/// DT_DEBUG 是否为间接跳转
pub(crate) const DT_DEBUG_INDIRECT: bool = false;

/// 是否需要为 DT_DEBUG 做重定位
pub(crate) const DT_DEBUG_INDIRECT_REL: bool = false;

/// FDPIC 常量位移标志位掩码
pub(crate) const FDPIC_CONSTDISP_FLAG: u32 = 0;
```

[Visibility]: Internal — rusl 编译期功能开关，控制条件编译路径。

#### 功能意图 (Intent)

与 C 版本对应，这些常量在编译期控制动态链接器的行为路径。Rust 版本使用 `cfg(feature = "...")` 或其他 `#[cfg]` 属性替代 C 的 `#ifndef` / `#define` 模式，语义更清晰且类型安全。

---

### 顶层常量

```rust
// Rust 声明
/// ELF 辅助向量最大条目数
pub(crate) const AUX_CNT: usize = 32;

/// .dynamic 段中 DT_* 标签最大数量
pub(crate) const DYN_CNT: usize = 37;
```

[Visibility]: Internal — rusl 动态链接器内部常量。

#### 功能意图 (Intent)

- `AUX_CNT = 32`: ELF 辅助向量的最大条目数。
- `DYN_CNT = 37`: `.dynamic` 段中动态标签的最大数量，用于预分配数组大小。

---

### `Stage2Func` 类型

```rust
// Rust 声明 — 动态链接器第二阶段入口的函数指针类型
pub(crate) type Stage2Func = extern "C" fn(base: *mut u8, dso_count: *mut usize);
```

[Visibility]: Internal — rusl 动态链接器内部类型。

#### 功能意图 (Intent)

动态链接器第二阶段（stage 2）入口的函数指针类型。Stage 1 负责自举重定位，Stage 2 负责加载所有依赖共享库并执行全部重定位。

#### 参数说明

- `base`: 指向共享库加载基址的指针。
- `dso_count`: 指向运行时 DSO 计数的指针（用于在加载新库时递增）。

---

### `__dlsym`

```rust
// Rust 声明 — 内部符号查找函数
pub(crate) extern "C" fn __dlsym(
    handle: *mut c_void,
    name: *const c_char,
    caller_ctx: *mut c_void,
) -> *mut c_void;
```

> 保持 `extern "C"` ABI 以兼容 musl libc 其他模块的直接调用。在纯 rusl 实现中，若调用链路完全在 Rust 内部，可改用安全封装版本。

[Visibility]: Internal — rusl 内部符号查找函数，但通过 `dlsym` 公共实现间接暴露。

#### 功能意图 (Intent)

在动态链接器或已加载共享库中查找符号。与公共 `dlsym` 相比，`__dlsym` 接受第三个参数（调用者的 DSO 上下文），支持从特定模块的视角查找符号。

#### 前置条件 (Preconditions)

- **PRE-1**: `handle` 为有效的 DSO 句柄：`RTLD_DEFAULT`（全局搜索）、`RTLD_NEXT`（调用者之后搜索）、或 `dlopen` 返回的句柄。
- **PRE-2**: `name` 为以 null 结尾的符号名字符串。
- **PRE-3**: `caller_ctx` 为调用者上下文（通常是调用模块的 DSO 指针），用于 `RTLD_NEXT` 和符号可见性计算。

#### 后置条件 (Postconditions)

- **Case 1 (成功)**: 返回符号地址；无错误状态。
- **Case 2 (符号未找到)**: 返回 `null`；通过 `__dl_seterr` 设置错误消息；`dlerror()` 返回描述性错误字符串。

---

### `__dl_seterr`

```rust
// Rust 声明 — 设置动态链接器错误消息
pub(crate) fn __dl_seterr(msg: &str);
```

[Visibility]: Internal — rusl 动态链接器内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

设置动态链接器错误消息。内部维护一个线程局部的错误缓冲区，供 `dlerror()` 读取。

#### 前置条件 (Preconditions)

- **PRE-1**: `msg` 为有效的字符串切片。

#### 后置条件 (Postconditions)

- **POST-1**: 错误消息存入线程局部的错误缓冲区。
- **POST-2**: 下一次 `dlerror()` 调用将返回该消息并清空缓冲区。

#### Rust 实现建议

使用线程局部存储（`#[thread_local]` 与 `RefCell<String>` 或 `UnsafeCell`）存储错误消息，避免 C 的 `va_list` 格式化机制。

---

### `__dl_invalid_handle`

```rust
// Rust 声明 — 检查 DSO 句柄有效性
pub(crate) fn __dl_invalid_handle(handle: *mut c_void) -> bool;
```

[Visibility]: Internal — rusl 动态链接器内部函数。

#### 功能意图 (Intent)

检查给定的 DSO 句柄是否是有效的 `dlopen` 句柄。用于 `dlclose`、`dlsym` 等函数中的参数验证。

#### 前置条件 (Preconditions)

- **PRE-1**: `handle` 是用户通过 `dlopen` 获得的句柄或 `RTLD_DEFAULT`/`RTLD_NEXT` 特殊值。

#### 后置条件 (Postconditions)

- **POST-1**: 若句柄有效，返回 `false`。
- **POST-2**: 若句柄无效，返回 `true`；调用者通常设置 `errno` 并返回错误。

---

### `__tlsdesc_static` / `__tlsdesc_dynamic`

```rust
// Rust 声明 — TLS 描述符解析函数
// 注意：这两个函数具有架构特定的调用约定，
// 通常由 GCC/Clang 生成的 TLS 描述符调用点直接调用。
// 在 Rust 中，它们可能通过 #[naked] 或 global_asm! 实现。

// 架构特定——通常通过 global_asm! 实现
// extern "C" { pub(crate) fn __tlsdesc_static() -> isize; }
// extern "C" { pub(crate) fn __tlsdesc_dynamic() -> isize; }
```

[Visibility]: Internal — rusl TLS 描述符解析函数。必须保持 `extern "C"` 可见符号，因为 GCC/Clang 生成的代码直接引用这些符号名。

#### 功能意图 (Intent)

TLS 描述符的两个解析函数：

- **`__tlsdesc_static`**: 解析静态 TLS 模型的访问——TLS 变量在主程序或启动时加载的共享库中定义，偏移量已知且固定。
- **`__tlsdesc_dynamic`**: 解析动态 TLS 模型的访问——TLS 变量在 `dlopen` 加载的共享库中定义，偏移量在运行时通过 `__tls_get_addr` 获取。

两个函数均无显式参数（使用 TLS 描述符的隐式参数寄存器/栈传递机制，取决于架构 ABI）。

#### 后置条件 (Postconditions)

- **POST-1**: 返回目标 TLS 变量相对于线程指针的偏移量。
- **POST-2**: 对于 `__tlsdesc_static`，结果在加载时确定且不变。
- **POST-3**: 对于 `__tlsdesc_dynamic`，首次访问时可能触发 TLS 块分配。

#### Rust 实现说明

这些函数通常需要 `#[naked]` 函数（不稳定特性）或 `global_asm!` 宏实现，因为它们的调用约定由架构 ABI 的 TLS 描述符规范定义，而非标准 C 调用约定。

---

### `__malloc_replaced` / `__aligned_alloc_replaced`

```rust
// Rust 声明 — 指示用户是否替换了 malloc 实现
pub(crate) static __malloc_replaced: AtomicBool = AtomicBool::new(false);
pub(crate) static __aligned_alloc_replaced: AtomicBool = AtomicBool::new(false);
```

[Visibility]: Internal — rusl 内部全局标志变量。

#### 功能意图 (Intent)

指示用户程序是否通过 `LD_PRELOAD` 或静态链接替换了 musl 的 `malloc`/`aligned_alloc` 实现。动态链接器在加载时检测符号冲突并设置这些标志。

#### 不变量 (Invariants)

- **INV-1**: `__malloc_replaced == false` 表示 libc 内部可以使用直接路径调用 `malloc`。
- **INV-2**: `__malloc_replaced == true` 表示用户替换了 `malloc`，内部代码必须通过 PLT 调用。

#### Rust 实现说明

在纯 Rust 静态链接环境中，此机制通常不需要（Rust 无 `LD_PRELOAD` 的等效语义）。仅在 rusl 作为 C 共享库替代时启用。

---

### `__malloc_donate`

```rust
// Rust 声明 — 将内存区域捐赠给 malloc 实现
pub(crate) fn __malloc_donate(start: *mut u8, end: *mut u8);
```

[Visibility]: Internal — rusl 内部内存管理函数。

#### 功能意图 (Intent)

将一块连续内存区域"捐赠"给 rusl 的 malloc 实现。用于动态链接器将不再需要的内存区域回收到 malloc 的内存池。

#### 前置条件 (Preconditions)

- **PRE-1**: `[start, end)` 是一块有效的、无别名连续内存区域。
- **PRE-2**: 该区域当前不被任何其他代码使用。
- **PRE-3**: `start <= end`。

#### 后置条件 (Postconditions)

- **POST-1**: 该内存区域被纳入 malloc 内部空闲链表，可供后续 `malloc` 分配使用。

---

### `__malloc_allzerop`

```rust
// Rust 声明 — 检查内存块是否全零
pub(crate) fn __malloc_allzerop(ptr: *const u8, size: usize) -> bool;
```

[Visibility]: Internal — rusl 内部内存管理函数。

#### 功能意图 (Intent)

检查给定的内存块是否所有字节均为零。用于 `calloc` 的快速路径：当 `calloc` 从 `malloc` 获得的内存恰好全零时，可以跳过显式清零操作。

#### 前置条件 (Preconditions)

- **PRE-1**: `ptr` 指向有效的已分配内存块。
- **PRE-2**: `size` 为该内存块的大小（字节）。

#### 后置条件 (Postconditions)

- **POST-1**: 若内存块所有字节为 0，返回 `true`。
- **POST-2**: 若存在任何非零字节，返回 `false`。

---

## 全局不变量

- **GINV-1 (ELF 类型一致性)**: `ElfEhdr`、`ElfPhdr`、`ElfSym` 的类型定义必须与目标 ABI 的字长一致，由 `target_pointer_width` 编译期保证。
- **GINV-2 (重定位类型兼容)**: 所有架构模块中定义的重定位类型必须映射到 `RelType` 中的对应名称，且不产生常量冲突。
- **GINV-3 (TLS 安全)**: `__tlsdesc_static` 和 `__tlsdesc_dynamic` 必须在任何 TLS 访问之前被正确注册到 TLS 描述符调用点中。
- **GINV-4 (错误消息线程安全)**: `__dl_seterr` 存储的错误消息是线程局部的，一个线程设置错误不影响另一个线程的 `dlerror()` 结果。
- **GINV-5 (#[no_std] 兼容)**: 本模块不依赖任何 Rust std 库，所有功能通过 `core` 和 rusl 内部模块实现。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `ElfEhdr` / `ElfPhdr` / `ElfSym` | `crate::internal::elf` | ELF 结构体定义 |
| `arch::*` (REL_* 常量) | `crate::internal::arch::{target}` | 架构特定重定位类型 |
| `__dlsym` 实现 | `src/ldso/mod.rs` | 动态链接器核心实现 |
| `__dl_seterr` 实现 | `src/ldso/mod.rs` | 错误消息管理 |
| `__dl_invalid_handle` 实现 | `src/ldso/mod.rs` | 句柄验证 |
| `__tlsdesc_static` / `__tlsdesc_dynamic` | `src/ldso/` 或 `global_asm!` | TLS 解析 |
| `__malloc_replaced` / `__aligned_alloc_replaced` | `src/ldso/mod.rs` | 符号替换检测 |
| `__malloc_donate` / `__malloc_allzerop` | `src/malloc/mod.rs` | malloc 内部接口 |

---

## Rust 与 C 实现关键差异总结

| 方面 | C (musl dynlink.h) | Rust (rusl dynlink) |
|------|-------------------|---------------------|
| ELF 类型选择 | `UINTPTR_MAX == 0xffffffff` 运行时常量 | `#[cfg(target_pointer_width = "32")]` 编译期属性 |
| 重定位提取 | 宏 `R_TYPE(x)` / `R_SYM(x)` | 函数 `r_type(x)` / `r_sym(x)`（编译期内联） |
| 重定位默认值 | 负整数枚举（`-100` 起始） | `u32::MAX - 99` 起始的正数枚举 |
| FDPIC | 由架构头文件提供 `#ifndef` 守卫 | `#[cfg(feature = "fdpic")]` feature gate |
| 可变参数格式化 | `va_list` 格式字符串 | `&str` 或 `format_args!` |
| 全局标志变量 | `extern hidden int` / `volatile` | `static AtomicBool` |
| TLS 描述符 | `hidden ptrdiff_t` 声明 + 汇编实现 | `global_asm!` 或 `#[naked]` 实现 |