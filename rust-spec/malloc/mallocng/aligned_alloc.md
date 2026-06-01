# aligned_alloc Rust 接口

## 复杂度分级: Level 3

> C 源文件: `src/malloc/mallocng/aligned_alloc.c`
> 对应 C spec: `src/malloc/mallocng/spec/aligned_alloc.md`

---

## [RELY]

```
aligned_alloc (对外导出, extern "C")
  ├── crate::malloc::malloc
  │     └── pub unsafe extern "C" fn malloc(n: usize) -> *mut c_void;
  │         // 底层分配器, 来自 malloc 模块
  │
  ├── crate::malloc::meta (内部模块, 重新设计)
  │     ├── struct Meta { ... }
  │     │     // 元数据结构体 (原 struct meta)
  │     │     // 字段: prev, next (链表指针), mem (关联 Group),
  │     │     //       avail_mask, freed_mask (位掩码),
  │     │     //       last_idx:5, freeable:1, sizeclass:6, maplen
  │     │
  │     ├── struct Group { ... }
  │     │     // 内存组结构体 (原 struct group)
  │     │     // 字段: meta (指向 Meta), active_idx:5, storage[]
  │     │
  │     ├── const UNIT: usize = 16;
  │     │     // 基本对齐单位, 内部常量
  │     │
  │     ├── const IB: usize = 4;
  │     │     // In-band 头部大小, 内部常量
  │     │
  │     ├── unsafe fn get_meta(p: *const u8) -> &Meta;
  │     │     // 从分配指针反查元数据, 含多重安全断言
  │     │     // 前置: p 为 16 字节对齐的有效分配指针
  │     │     // 后置: 返回 p 所属组的 Meta 引用; 断言失败则 abort
  │     │
  │     ├── fn get_slot_index(p: *const u8) -> usize;
  │     │     // 从 p[-3] 提取槽位索引 (低 5 位, 0..31)
  │     │     // 前置: p 为有效分配指针
  │     │
  │     ├── fn get_stride(m: &Meta) -> usize;
  │     │     // 计算组内每个槽位的跨步大小
  │     │     // Case 1 (mmap 单槽位): maplen * PGSZ - UNIT
  │     │     // Case 2 (常规 slab 组): UNIT * SIZE_CLASSES[m.sizeclass]
  │     │
  │     ├── unsafe fn set_size(p: *mut u8, end: *mut u8, n: usize);
  │     │     // 在分配块的 in-band header 中写入用户请求大小 n
  │     │     // 通过设置 reserved 字段实现
  │     │
  │     └── static SIZE_CLASSES: [u16; 48];
  │           // 48 个大小类别的槽位容量表 (以 UNIT 为单位)
  │           // 定义于 malloc 模块
  │
  ├── crate::malloc::glue (内部模块)
  │     └── fn is_aligned_alloc_disabled() -> bool;
  │           // 检查 aligned_alloc 是否被禁用
  │           // 等价于 C 侧 DISABLE_ALIGNED_ALLOC 宏
  │           // 依赖 __malloc_replaced && !__aligned_alloc_replaced
  │
  ├── crate::dynlink (内部模块, 简化设计)
  │     ├── static __malloc_replaced: AtomicBool;
  │     │     // 标记 malloc 是否被外部替换
  │     │     // rusl no_std 环境下始终为 false
  │     │
  │     └── static __aligned_alloc_replaced: AtomicBool;
  │           // 标记 aligned_alloc 是否被外部替换
  │           // rusl no_std 环境下始终为 false
  │
  ├── 外部常量 / 错误机制
  │     ├── core::usize::MAX  (等价于 C 的 SIZE_MAX)
  │     ├── errno 全局错误码机制
  │     ├── EINVAL   // 参数无效
  │     └── ENOMEM   // 内存不足
  │
  └── 递归依赖终止
        ├── malloc() — 来自 crate::malloc, 其规约在 malloc.md 的 Rust spec 中独立描述
        ├── errno / EINVAL / ENOMEM — C 标准库全局 errno 机制, 外部模块
        ├── core::usize::MAX — Rust 语言内建常量, 等价于 C SIZE_MAX
        ├── get_meta / get_slot_index / get_stride / set_size — meta 模块内部函数
        ├── Meta / Group / UNIT / IB — meta 模块内部类型和常量
        ├── SIZE_CLASSES — malloc 模块内部数组
        ├── is_aligned_alloc_disabled — glue 模块函数, 其规约见 glue.h 的 spec
        └── __malloc_replaced / __aligned_alloc_replaced — dynlink 模块内部标志
```

---

## [GUARANTEE]

### 对外导出接口

```rust
// [Visibility]: Public — POSIX 标准函数, <stdlib.h> 声明
// [ABI Compatibility]: extern "C", 参数布局与原 C 接口完全兼容
#[no_mangle]
pub unsafe extern "C" fn aligned_alloc(align: usize, len: usize) -> *mut core::ffi::c_void;
```

#### 前置条件

1. **对齐要求**: `align` 必须是 2 的幂 (`(align & align.wrapping_neg()) == align`)，否则调用失败。
2. **大小要求**: `len` 必须是 `align` 的整数倍（POSIX 标准要求，本实现不做显式检查，但行为正确）。
3. **溢出检查**: `len + align` 必须不超过 `usize::MAX`（`len <= usize::MAX - align`）。
4. **对齐上限**: `align` 必须小于 `(1u64 << 31) * UNIT` 即 `2^31 * 16 = 32 GB`（在 64 位平台上: `align < (1usize << 31) * 16`）。
5. **分配器可用**: `is_aligned_alloc_disabled()` 必须返回 `false`（rusl `no_std` 环境下始终为 `false`，因不存在动态链接器替换）。

#### 后置条件

**Case 1 (成功)**: 返回一个至少 `len` 字节的已分配内存块指针 `p`，满足：
- `(p as usize) % align == 0`（地址对齐到 `align` 边界）
- 内存块可安全写入 `len` 字节
- 分配块属于 mallocng 管理的某个 `Group`，具有完整的元数据头部
- `p` 可通过标准 `free(p)` 安全释放

**Case 2 (失败)**: 返回 `core::ptr::null_mut()`，并设置 `errno`：
- `errno = EINVAL`：当 `align` 不是 2 的幂
- `errno = ENOMEM`：当 `len` 溢出、`align` 过大、`aligned_alloc` 被禁用、或底层 `malloc` 分配失败

#### 不变量

- **对齐不变量**: 返回的指针 `p` 始终满足 `(p as usize) % align == 0`（当 `align <= UNIT` 时，`align` 被提升为 `UNIT = 16`）。
- **元数据不变量**: 返回的 `p` 必须能被 `get_meta(p)` 正确解析，即头部字段与组结构一致。
- **槽位边界不变量**: `p.add(len) <= end`，即用户可用空间不超过槽位的实际存储空间。
- **offset 一致性**: `p` 头部记录的偏移值乘以 `UNIT` 加上 `UNIT`（Group 头部大小）必须能定位到 `Group::storage`。

#### 系统算法 (Level 3)

`aligned_alloc` 的实现策略是 **过度分配 (over-allocate) 然后内部偏移 (internal offset)** ，而非请求 OS 直接提供对齐内存：

**阶段 1 -- 参数校验**:
```
if (align & align.wrapping_neg()) != align => errno = EINVAL, return null
if len > usize::MAX - align || align >= (1usize << 31) * UNIT => errno = ENOMEM, return null
if is_aligned_alloc_disabled() => errno = ENOMEM, return null
if align <= UNIT => align = UNIT  // 最小对齐为 16 字节
```
`(align & align.wrapping_neg()) != align` 是经典的 2 的幂判定：对 2 的幂 `n`，`n & -n == n`（补码性质）。

**阶段 2 -- 过度分配**:
```
p = malloc(len + align - UNIT)
```
分配比请求多 `align - UNIT` 字节的空间。由于 `malloc` 返回的指针已经是 16 字节对齐的，最坏情况下需要额外 `align - UNIT` 字节来保证能将指针提升到 `align` 对齐边界。

**阶段 3 -- 获取槽位布局信息**:
```
g = get_meta(p)          // 获取元数据
idx = get_slot_index(p)  // 获取槽位索引
stride = get_stride(g)   // 获取槽位跨度
start = g.mem.storage_ptr().add(stride * idx)       // 槽位起始地址
end   = g.mem.storage_ptr().add(stride * (idx + 1) - IB)  // 槽位末尾地址
adj   = (-(p as isize) as usize) & (align - 1)      // 需要向上调整的字节数
```
`adj` 的计算：`(-(p as isize) as usize) & (align - 1)` 等价于 `(align - (p as usize) % align) % align`。

**阶段 4a -- 已对齐的快速路径**:
```
if adj == 0 {
    set_size(p, end, len);
    return p;
}
```
若 `malloc` 返回的地址恰好在 `align` 边界上，无需任何调整。

**阶段 4b -- 偏移调整并重写头部**:
```
p = p.add(adj);                                  // 将指针偏移到对齐位置
offset = (p as usize - g.mem.storage_addr()) / UNIT;  // 计算新偏移 (单位: UNIT)
```
然后根据偏移量大小选择头部编码格式：

**小偏移 (<= 0xffff) -- 16-bit 编码**:
```
p[-2..-1] 写入 u16 LE = offset     // 16-bit 偏移
p[-4] = 0                           // 标志: 使用 16-bit 偏移
```

**大偏移 (> 0xffff) -- 32-bit 编码**:
```
p[-2..-1] 写入 u16 LE = 0          // 必须为 0
p[-8..-5] 写入 u32 LE = offset     // 32-bit 偏移
p[-4] = 1                           // 标志: 使用 32-bit 偏移
```

设置槽位索引和大小:
```
p[-3] = idx as u8                    // 低 5 位记录槽位索引
set_size(p, end, len)                // 记录分配大小 (可能覆盖 p[-3] 高 3 位)
```

**阶段 5 -- 在原槽位头部写入"对齐 enframing"信息**:
```
*(u16 *)(start - 2) = (p - start) / UNIT   // 新位置相对原槽位起点的偏移
start[-3] = 7 << 5                           // 设置预留大小 = 7 (最大值)
```

这一步在原 `malloc` 返回的地址 `start` 对应的头部写入信息，标记了该区域是"被 aligned_alloc 偏移过的"，便于调试和堆遍历工具找到实际的对齐分配位置。

#### 复杂度

- **时间复杂度**: O(1) -- 除 `malloc` 调用外，所有操作均为常数时间的指针运算和元数据读写。
- **空间开销**: 最多额外分配 `align - UNIT` 字节（用于对齐调整）。小对齐（如 32、64 字节）时开销极小；极端对齐（如 4KB）时开销接近一页。

#### 与 C11/POSIX 标准的关系

`aligned_alloc` 是 C11 标准引入的函数，POSIX.1-2017 采用。标准要求：
1. `align` 必须是 2 的幂
2. `len` 必须是 `align` 的整数倍
3. 返回的内存可通过 `free()` 释放

本实现满足上述所有要求。

---

## 内部依赖符号汇总

| 符号 | Rust 类型/表示 | 来源模块 | 可见性 |
|------|---------------|---------|--------|
| `aligned_alloc` | `extern "C" fn(usize, usize) -> *mut c_void` | aligned_alloc 模块 | **Public** `<stdlib.h>` |
| `malloc` | `extern "C" fn(usize) -> *mut c_void` | malloc 模块 | **Public** `<stdlib.h>` |
| `errno` | 全局可写变量 (thread-local) | 错误处理模块 | **Public** `<errno.h>` |
| `EINVAL` | 常量 `i32` | 错误处理模块 | **Public** `<errno.h>` |
| `ENOMEM` | 常量 `i32` | 错误处理模块 | **Public** `<errno.h>` |
| `SIZE_MAX` | `core::usize::MAX` | Rust 语言内建 | Public |
| `UNIT` | `const usize = 16` | meta 模块 | Internal |
| `IB` | `const usize = 4` | meta 模块 | Internal |
| `is_aligned_alloc_disabled` | `fn() -> bool` | glue 模块 | Internal |
| `Meta` | `struct Meta` | meta 模块 | Internal |
| `Group` | `struct Group` | meta 模块 | Internal |
| `get_meta` | `unsafe fn(*const u8) -> &Meta` | meta 模块 | Internal |
| `get_slot_index` | `fn(*const u8) -> usize` | meta 模块 | Internal |
| `get_stride` | `fn(&Meta) -> usize` | meta 模块 | Internal |
| `set_size` | `unsafe fn(*mut u8, *mut u8, usize)` | meta 模块 | Internal |
| `SIZE_CLASSES` | `static [u16; 48]` | malloc 模块 | Internal |
| `__malloc_replaced` | `static AtomicBool` | dynlink 模块 | Internal |
| `__aligned_alloc_replaced` | `static AtomicBool` | dynlink 模块 | Internal |

---

## 跨文件依赖说明

| 依赖符号 | 来源文件 | 说明 |
|---------|---------|------|
| `malloc()` | `malloc.rs` (mallocng) | 底层分配器函数 |
| `Meta` / `Group` / `UNIT` / `IB` | `meta.rs` (mallocng) | 核心数据结构和常量 |
| `get_meta()` / `get_slot_index()` / `get_stride()` / `set_size()` | `meta.rs` (mallocng) | 内部辅助函数 |
| `SIZE_CLASSES[]` | `malloc.rs` (mallocng) | 大小类别查找表 |
| `is_aligned_alloc_disabled()` | `glue.rs` (mallocng) | 分配器禁用检测 |
| `__malloc_replaced` / `__aligned_alloc_replaced` | `dynlink.rs` | 动态链接替换标志 |
| `errno` / `EINVAL` / `ENOMEM` | 错误处理模块 | POSIX 错误码机制 |

---

## rusl no_std 适配说明

1. **无 `libc` crate**: 所有 C ABI 类型使用 `core::ffi::c_void`、`usize`（等价 `size_t`）、`i32`（等价 `c_int`）等 Rust 原生类型。
2. **no_std 约束**: 不依赖 `std::alloc`，内部使用 mallocng 自己的分配器；`core::ptr::null_mut()` 替代 `std::ptr::null_mut()`。
3. **`__malloc_replaced` 简化**: rusl 为 `#![no_std]` 静态链接库，不存在动态链接器替换 `malloc` 的场景，因此 `is_aligned_alloc_disabled()` 始终返回 `false`。`__malloc_replaced` 和 `__aligned_alloc_replaced` 可以保留为 `AtomicBool` 常量（始终为 `false`），或直接从 rusl 实现中省略。
4. **`errno` 机制**: rusl 需自行实现 thread-local `errno` 存储及 `EINVAL`/`ENOMEM` 常量定义，不依赖外部 `libc` 的 `__errno_location`。
5. **内联汇编替代 syscall 封装**: `malloc` 内部调用的 `mmap`/`brk` 等系统调用由 rusl 通过 `asm!` 直接发起，不经过 `libc` crate。