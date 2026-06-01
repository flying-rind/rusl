# memalign 规约 (Rust 接口)

> 对应 C 源文件: `src/malloc/memalign.c`
> 符号数量: 1 对外导出，0 内部私有
> rusl 模块路径: `rusl/src/malloc/memalign.rs`

---

## 依赖图

```
memalign (对外导出, #[no_mangle] extern "C")
  └── aligned_alloc (内部委托)
        ├── [mallocng 路径] ├── malloc (内部依赖, malloc.rs)
        │                    ├── meta 模块 (内部依赖, meta.rs)
        │                    │    ├── struct Meta / struct Group
        │                    │    ├── Unit / Ib 常量
        │                    │    ├── get_meta / get_slot_index / get_stride / set_size
        │                    │    └── size_classes 全局表
        │                    ├── glue 模块 (内部依赖, glue.rs)
        │                    │    ├── DisableAlignedAlloc 条件编译宏
        │                    │    ├── __malloc_replaced / __aligned_alloc_replaced (标志变量)
        │                    │    └── 锁原语 (wrlock / unlock)
        │                    └── syscall 模块 (内部依赖, syscall.rs)
        │                         └── mmap / munmap / madvise / brk
        │
        └── [oldmalloc 路径] ├── malloc (内部依赖, oldmalloc/malloc.rs)
                             ├── __bin_chunk (内部依赖, oldmalloc/malloc.rs)
                             ├── malloc_impl 模块 (内部依赖, oldmalloc/malloc_impl.rs)
                             │    ├── struct Chunk
                             │    ├── 宏: SIZE_ALIGN, C_INUSE, IS_MMAPPED, MEM_TO_CHUNK, NEXT_CHUNK
                             │    └── OVERHEAD 常量
                             ├── __malloc_replaced / __aligned_alloc_replaced (标志变量)
                             └── syscall 模块 (内部依赖, syscall.rs)
```

> 注：rusl 在编译期通过 feature flag 选择 mallocng 或 oldmalloc 分配器实现，`memalign` 本身与具体分配器解耦。

---

## memalign (对外导出)

```rust
#[no_mangle]
pub extern "C" fn memalign(align: usize, len: usize) -> *mut core::ffi::c_void;
```

**[Visibility]: External (Public)** — musl `<malloc.h>` 第17行及 `<stdlib.h>` 第145行（条件编译）声明，源自 SunOS/BSD 的遗存函数，POSIX.1-2008 标记为 obsolescent。rusl 中通过 `#[no_mangle] extern "C"` 保持 ABI 兼容性，外部 C 代码可透明调用。

### 意图 (Intent)

提供按指定对齐边界分配堆内存的能力。rusl 将其实现为内部 `aligned_alloc(align, len)` 的直接委托——无任何适配层或参数变换。

### 前置条件 (Preconditions)

1. **对齐参数**：`align` 必须是 2 的幂（`align & align.wrapping_neg() == align`），否则内部 `aligned_alloc` 返回 `null_mut()` 并设置 `errno = EINVAL`。
2. **大小参数**：`len` 必须满足 `len <= usize::MAX - align`，且 `align < (1u64 << 31) * UNIT`，否则内部 `aligned_alloc` 返回 `null_mut()` 并设置 `errno = ENOMEM`。
3. **替换检测**：若 `malloc` 被外部替换（`__malloc_replaced != 0`）且 `aligned_alloc` 未被一同替换（`__aligned_alloc_replaced == 0`），则内部 `aligned_alloc` 返回 `null_mut()` 并设置 `errno = ENOMEM`。
   - 注：此机制依赖 ELF 符号插替检测，rusl 若仅作为静态链接库使用可简化或移除该检测。
4. **对齐下界**：若 `align <= UNIT`（malloc-ng 内部最小对齐单元），调用方传入的 `align` 值被提升至 `UNIT`（实际上等价于普通 `malloc`）。

### 后置条件 (Postconditions)

| 分支 | 条件 | 结果 |
|------|------|------|
| 成功 | 内部 `aligned_alloc(align, len)` 成功 | 返回指向至少 `len` 字节、地址对齐于 `align` 边界的内存块指针（`*mut c_void`）。内存内容未初始化。 |
| 失败 | 内部 `aligned_alloc(align, len)` 返回 `null_mut()` | 返回 `null_mut()`，`errno` 被设置为 `EINVAL` 或 `ENOMEM`（取决于失败原因）。 |

### 系统算法 (System Algorithm)

**Level 3** — 实现策略至关重要。

`memalign` 采用 **委托模式 (Delegation Pattern)**：

```
memalign(align, len)
  = aligned_alloc(align, len)   // 内部实现，定义于分配器 crate 中
                                // mallocng 路径: rusl/src/malloc/mallocng/aligned_alloc.rs
                                // oldmalloc 路径: rusl/src/malloc/oldmalloc/aligned_alloc.rs
```

此实现策略选择具有两层含义：

1. **语义收窄**：传统 BSD `memalign` 允许 `len` 不为 `align` 的整数倍，但 C11 `aligned_alloc` 有此要求。rusl 的 malloc-ng 版 `aligned_alloc` 不显式校验 `len % align == 0`，因此行为上等价于传统 BSD 版本。
2. **分配器切换透明**：rusl 通过 Cargo feature flag 在编译期选择 malloc-ng 或 oldmalloc 分配器实现，`memalign` 调用对应的内部 `aligned_alloc` 而无需感知差异，实现了源码级别的分配器无关性。

**Rust 实现策略**：

```
// memalign.rs — 薄封装层
use crate::malloc::aligned_alloc;  // 由 feature flag 决定具体实现

#[no_mangle]
pub extern "C" fn memalign(align: usize, len: usize) -> *mut c_void {
    aligned_alloc(align, len)
}
```

`aligned_alloc` 本身也是 `extern "C"` 的公共 API，在 rusl 内部它是 `pub(crate)` 可见、直接被 `memalign` 调用。Rust 的类型系统和所有权模型使得委托调用自然是安全的——`memalign` 不持有任何状态，仅作为 C ABI 入口点转发调用。

### 不变量 (Invariants)

无模块局部不变量。该函数为纯委托，所有不变量由 `aligned_alloc` 的内部实现维护。

### 错误码

| errno 值 | 触发条件 |
|----------|----------|
| `EINVAL` | `align` 不是 2 的幂 |
| `ENOMEM` | `len > usize::MAX - align` 或 `align` 过大或分配器已被替换但 `aligned_alloc` 未被替换或底层 `malloc` 返回 `null_mut()` |

### 边界情况

- **align = 0**：不满足 2 的幂条件，被视为非法参数，`aligned_alloc` 返回 `null_mut()` 并设 `errno = EINVAL`。
- **len = 0**：行为由底层 `aligned_alloc` 决定。C 标准允许 `malloc(0)` 返回 NULL 或可安全传给 `free()` 的非 NULL 指针；rusl 的实现与 musl 行为一致。
- **超大对齐**：若 `align >= (1u64 << 31) * UNIT`，`aligned_alloc` 直接返回 `null_mut()` + `ENOMEM`，即使系统有足够内存也不尝试分配。这是对极端对齐请求的硬性拒绝。

---

## ABI 兼容性说明

该函数通过 `#[no_mangle] extern "C"` 对外导出，参数和返回值布局完全兼容原 C ABI：

| C 类型 | Rust 类型 | 大小 (64-bit) | 对齐 (64-bit) | 说明 |
|--------|-----------|---------------|---------------|------|
| `size_t` | `usize` | 8 字节 | 8 字节 | ABI 兼容 |
| `void *` | `*mut c_void` | 8 字节 | 8 字节 | 指针类型，布局一致 |
| 返回值 `void *` | `*mut c_void` | 8 字节 | 8 字节 | 返回值通过 RAX 寄存器传递 |

**调用约定**: `extern "C"` 确保使用 System V AMD64 ABI（Linux x86_64）或对应平台的 C 调用约定，与原 musl 行为一致。

---

## Rust 安全抽象层（rusl 内部可选）

`memalign` 本身作为 `extern "C"` 函数必须是 `unsafe` 语义（返回原始指针）。但 rusl 可在内部提供一个安全的 Rust 封装（`pub(crate)` 可见性），供其他 rusl 内部模块使用：

```rust
// 内部安全封装（非对外导出）
pub(crate) fn memalign_safe(align: usize, len: usize) -> Option<&'static mut [u8]> {
    let ptr = unsafe { memalign(align, len) };
    if ptr.is_null() {
        None
    } else {
        // Safety: memalign 返回的内存由分配器保证有效性
        Some(unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, len) })
    }
}
```

此安全封装仅在 rusl 内部使用，不导出，不进入 `[GUARANTEE]`。

---

## 跨文件依赖说明

| 依赖符号 | 定义位置 (rusl) | 性质 |
|----------|-----------------|------|
| `aligned_alloc` | `rusl/src/malloc/mallocng/aligned_alloc.rs` 或 `rusl/src/malloc/oldmalloc/aligned_alloc.rs` | 内部 `pub(crate)` 函数，同时对外以 `extern "C"` 导出 |
| `__malloc_replaced` | `rusl/src/malloc/replaced.rs`（若保留）或编译期常量 | Internal — 替换检测标志 |
| `__aligned_alloc_replaced` | `rusl/src/malloc/replaced.rs`（若保留）或编译期常量 | Internal — 替换检测标志 |

### 递归依赖追踪

```
memalign
  └── aligned_alloc (跨模块依赖)
        │
        ├── [mallocng 路径]
        │   ├── malloc                → rusl/src/malloc/mallocng/malloc.rs
        │   ├── UNIT / IB             → rusl/src/malloc/mallocng/meta.rs (内部常量)
        │   ├── struct Meta / Group   → rusl/src/malloc/mallocng/meta.rs (内部类型)
        │   ├── get_meta              → rusl/src/malloc/mallocng/meta.rs (内部 inline 函数)
        │   ├── get_slot_index        → rusl/src/malloc/mallocng/meta.rs (内部 inline 函数)
        │   ├── get_stride            → rusl/src/malloc/mallocng/meta.rs (内部 inline 函数)
        │   ├── set_size              → rusl/src/malloc/mallocng/meta.rs (内部 inline 函数)
        │   ├── size_classes          → rusl/src/malloc/mallocng/malloc.rs (内部全局数组)
        │   ├── errno / EINVAL / ENOMEM → rusl/src/errno.rs (内部模块)
        │   ├── DISABLE_ALIGNED_ALLOC → rusl/src/malloc/mallocng/glue.rs (内部宏/常量)
        │   ├── __malloc_replaced     → rusl/src/malloc/replaced.rs (标志变量)
        │   ├── __aligned_alloc_replaced → rusl/src/malloc/replaced.rs (标志变量)
        │   └── mmap                  → rusl/src/syscall/mman.rs (内部 syscall 封装)
        │
        └── [oldmalloc 路径]
            ├── malloc                → rusl/src/malloc/oldmalloc/malloc.rs
            ├── __bin_chunk           → rusl/src/malloc/oldmalloc/malloc.rs (内部函数)
            ├── SIZE_ALIGN            → rusl/src/malloc/oldmalloc/malloc_impl.rs (内部常量)
            ├── C_INUSE               → rusl/src/malloc/oldmalloc/malloc_impl.rs (内部常量)
            ├── IS_MMAPPED            → rusl/src/malloc/oldmalloc/malloc_impl.rs (内部宏)
            ├── MEM_TO_CHUNK          → rusl/src/malloc/oldmalloc/malloc_impl.rs (内部宏)
            ├── NEXT_CHUNK            → rusl/src/malloc/oldmalloc/malloc_impl.rs (内部宏)
            ├── struct Chunk          → rusl/src/malloc/oldmalloc/malloc_impl.rs (内部类型)
            ├── errno / EINVAL / ENOMEM → rusl/src/errno.rs (内部模块)
            ├── __malloc_replaced     → rusl/src/malloc/replaced.rs (标志变量)
            └── __aligned_alloc_replaced → rusl/src/malloc/replaced.rs (标志变量)
```

### 递归依赖终止说明

递归追踪在以下依赖处终止：

- **malloc()**：属于 rusl 内部分配器模块 — 其规约在 `malloc.rs` 的 spec 中独立描述
- **errno / EINVAL / ENOMEM**：rusl 内部 errno 机制 — 独立模块 `errno.rs` 提供
- **mmap / munmap / madvise / brk**：Linux syscall 封装 — rusl 内部 `syscall` 模块通过 `asm!` 内联汇编直接发起，不依赖 `libc` crate
- **`struct Meta` / `struct Group` / `UNIT` / `IB`**：malloc-ng 内部数据结构 — 已在 `meta.rs` / `meta.h` spec 中充分描述
- **`struct Chunk` / 相关宏**：oldmalloc 内部数据结构 — 已在 `malloc_impl.rs` / `malloc_impl.h` spec 中充分描述
- **`__malloc_replaced` / `__aligned_alloc_replaced`**：rusl 若仅作为静态库，可将这些标志简化为编译期常量（均为 0），避免运行时 ELF 符号查找开销

---

## 实现架构选择

rusl 中 `memalign` 的实现策略与 musl 完全一致：

```
                memalign.rs (薄封装层)
                      │
                      │ pub(crate) 调用
                      ▼
         ┌────────────────────────────┐
         │    aligned_alloc(align,len) │
         │  (分配器内部实现)            │
         └────────────────────────────┘
                      │
          ┌───────────┴───────────┐
          ▼                       ▼
   mallocng 路径            oldmalloc 路径
   (Cargo feature:         (Cargo feature:
    "malloc-mallocng")      "malloc-oldmalloc")
          │                       │
          ▼                       ▼
    过度分配 + 偏移          malloc + chunk 分裂
    + 元数据重写              + leading fragment 回收
```

两个路径通过 Cargo feature flag (`malloc-mallocng` 或 `malloc-oldmalloc`) 在编译期二选一，`memalign.rs` 自身不感知差异——它只依赖 `aligned_alloc` 符号，由条件编译决定实际链接哪个实现。

---

*本规约通过递归依赖追踪生成：`memalign` → `aligned_alloc`（跨文件依赖，终止追踪）。*