# realloc Rust 接口

## 复杂度分级: Level 3

> C 源文件: `src/malloc/mallocng/realloc.c`
> 对应 C spec: `src/malloc/mallocng/spec/realloc.md`
> POSIX 公共入口: `src/malloc/realloc.c` (薄封装，直接转发到内部实现)
> 实现架构说明: C 侧通过 `glue.h` 中 `#define realloc __libc_realloc` 将本文件定义的 `realloc` 重命名为 `__libc_realloc`。POSIX 公共入口 `src/malloc/realloc.c` 仅为薄封装，调用 `__libc_realloc`。**rusl 中取消此宏重命名层级**，直接在本模块实现 `pub unsafe extern "C" fn realloc` 作为对外导出符号，内部使用 `fn realloc_impl` 私有函数封装核心逻辑。

---

## [RELY]

```
realloc (对外导出, extern "C")
  ├── crate::malloc::malloc (对外导出模块, Public)
  │     └── pub unsafe extern "C" fn malloc(n: usize) -> *mut c_void;
  │         // 底层分配器, Case 1 (p 为空) 和 Case 5 (malloc+memcpy+free) 中调用
  │         // 其规约在 malloc.md 的 Rust spec 中独立描述
  │
  ├── crate::malloc::free (对外导出模块, Public)
  │     └── pub unsafe extern "C" fn free(p: *mut c_void);
  │         // 底层释放器, Case 5 中释放旧内存块
  │         // 其规约在 free.md 的 Rust spec 中独立描述
  │
  ├── crate::malloc::meta (内部模块, 完全重新设计)
  │     ├── struct Meta { ... }
  │     │     // 元数据结构体 (原 struct meta), Repr(C) 保证位域布局与 C ABI 兼容
  │     │     // 字段: prev: *mut Meta, next: *mut Meta (双向循环链表指针)
  │     │     //       mem: *mut Group (指向所属 Group 的指针)
  │     │     //       avail_mask: AtomicI32 (可用槽位位掩码)
  │     │     //       freed_mask: AtomicI32 (已释放槽位位掩码)
  │     │     //       last_idx: u64 (位域:5, 该组中最大槽位索引)
  │     │     //       freeable: u64 (位域:1, 标记是否可被整体回收)
  │     │     //       sizeclass: u64 (位域:6, 大小类别编号 0-47 或 63)
  │     │     //       maplen: u64 (位域:N, mmap 映射的页数)
  │     │     // rusl 重构: 将 volatile int 替换为 AtomicI32, 位域使用 bitflags 或手动位操作
  │     │
  │     ├── struct Group { ... }
  │     │     // 内存组结构体 (原 struct group), Repr(C) 保证内存布局兼容
  │     │     // 字段: meta: *mut Meta (指向所属元数据的反向指针)
  │     │     //       active_idx: u8 (位域:5, 当前活动掩码最高位编号 0..31)
  │     │     //       pad: [u8; UNIT - size_of::<*mut Meta>() - 1] (填充至 UNIT=16 字节)
  │     │     //       storage: [u8] (柔性数组/不定长, 实际存储区域)
  │     │     // rusl 重构: 柔性数组 storage[] 使用 DST (动态大小类型) 或裸指针偏移访问
  │     │
  │     ├── const UNIT: usize = 16;
  │     │     // 基本分配单元大小 (字节), 所有对齐的基础
  │     │
  │     ├── const IB: usize = 4;
  │     │     // In-band 头部大小 (字节), 槽位末尾保留的元数据/哨兵空间
  │     │
  │     ├── const MMAP_THRESHOLD: usize = 131052;
  │     │     // 超过此大小的分配使用独立 mmap 而非 slab 槽位分配
  │     │     // 在 Case 3 原地调整中要求 n < MMAP_THRESHOLD
  │     │     // 在 Case 4 mremap 路径中要求 n >= MMAP_THRESHOLD
  │     │
  │     ├── fn size_overflows(n: usize) -> bool;
  │     │     // 检查请求分配大小是否导致溢出
  │     │     // 前置: 无特定要求
  │     │     // 后置 Case 1 (溢出): 若 n >= SIZE_MAX/2 - 4096, 设置 errno = ENOMEM, 返回 true
  │     │     // 后置 Case 2 (正常): 返回 false, errno 不变
  │     │     // rusl 重构: SIZE_MAX 使用 core::usize::MAX
  │     │
  │     ├── unsafe fn get_slot_index(p: *const u8) -> usize;
  │     │     // 从分配指针 p 的 in-band header 中提取槽位索引
  │     │     // 前置: p 指向一个由 mallocng 分配的有效内存块起始地址
  │     │     // 后置: 返回 p[-3] & 31, 即槽位索引 (0-31)
  │     │     // rusl 重构: 使用 unsafe 指针运算, 返回 usize 而非 c_int
  │     │
  │     ├── unsafe fn get_meta(p: *const u8) -> &Meta;
  │     │     // 从用户指针逆向定位到管理该内存块的 struct meta
  │     │     // 前置: p 指向有效分配块的起始地址, (p as usize) 为 16 字节对齐
  │     │     // 后置 Case 1 (成功): 返回 p 所属组的 &Meta 引用
  │     │     // 后置 Case 2 (失败): 任一断言失败则 crash (a_crash / unreachable)
  │     │     // 校验链: 对齐检查 → 偏移量解析 → 组基址计算 → 双向绑定验证 →
  │     │     //         索引范围检查 → avail/freed_mask 检查 → meta_area.check 验证
  │     │     // rusl 重构: 断言改用 debug_assert! + unsafe { core::hint::unreachable_unchecked() }
  │     │     //          或统一使用 unreachable!() (Release 模式也保留检查)
  │     │
  │     ├── unsafe fn get_nominal_size(p: *const u8, end: *const u8) -> usize;
  │     │     // 从分配块的 header 中解码出原始分配给用户的大小 (old_size)
  │     │     // 前置: p 指向分配块起始, end = p + stride - IB
  │     │     // 后置: 返回 end - p - reserved, 即用户数据的实际可用大小
  │     │     // reserved 解码规则:
  │     │     //   reserved = p[-3] >> 5, 若 reserved < 5 直接使用
  │     │     //   若 reserved == 5, 从 end[-4..-1] 读取 u32 LE 扩展值
  │     │     // 断言: reserved <= end-p, *(end - reserved) == 0 (哨兵), *end == 0 (溢出检查)
  │     │     // rusl 重构: 使用 unsafe 指针读取, u32 LE 解码使用 u32::from_le_bytes
  │     │
  │     ├── fn get_stride(m: &Meta) -> usize;
  │     │     // 计算组内单个槽位的总跨度 (stride)
  │     │     // 前置: m 指向有效的 struct meta
  │     │     // 后置 Case 1 (mmap 单槽组): 若 m.last_idx == 0 && m.maplen != 0,
  │     │     //                           返回 m.maplen * 4096 - UNIT
  │     │     // 后置 Case 2 (常规 slab 组): 返回 UNIT * SIZE_CLASSES[m.sizeclass]
  │     │
  │     ├── unsafe fn set_size(p: *mut u8, end: *mut u8, n: usize);
  │     │     // 在分配块的 in-band header 中写入新的用户请求大小 n
  │     │     // 前置: p 指向分配块起始, end = p + stride - IB, n <= end - p
  │     │     // 后置: 新大小 n 被编码到隐藏头部 (reserved 字段)
  │     │     // 编码规则:
  │     │     //   reserved = end - p - n
  │     │     //   若 reserved > 0, end[-reserved] = 0 (哨兵)
  │     │     //   若 reserved >= 5, end[-4..-1] 写入 u32 LE 扩展值, end[-5] = 0 (哨兵)
  │     │     //   p[-3] = (p[-3] & 31) | ((reserved as u8) << 5) (低 5 位保留 slot index)
  │     │     // rusl 重构: 使用 unsafe 指针写入, u32 LE 编码使用 u32::to_le_bytes
  │     │
  │     ├── fn size_to_class(n: usize) -> usize;
  │     │     // 将用户请求大小 n 映射到大小类别索引 (0-47)
  │     │     // 前置: n 为用户请求的分配大小 (字节)
  │     │     // 后置: 返回 0..48 的 sizeclass 索引
  │     │     // 算法:
  │     │     //   n = (n + IB - 1) >> 4           — 转换为 UNIT 单位并向上取整
  │     │     //   若 n < 10, 直接返回 n           — 小对象精确匹配 (class 0-9)
  │     │     //   否则 n++, 使用 leading_zeros()  + SIZE_CLASSES 查表确定类别
  │     │     // rusl 重构: a_clz_32 替换为 u32::leading_zeros()
  │     │
  │     └── static SIZE_CLASSES: [u16; 48];
  │           // 48 个大小类别的槽位容量表 (以 UNIT 为单位)
  │           // 定义于 malloc 模块, 在 meta 模块中通过 extern 引用
  │           // 值: [1,2,3,4,5,6,7,8, 9,10,12,15, 18,20,25,31,
  │           //       36,42,50,63, 72,84,102,127, 146,170,204,255,
  │           //       292,340,409,511, 584,682,818,1023, 1169,1364,1637,2047,
  │           //       2340,2730,3276,4095, 4680,5460,6552,8191]
  │
  ├── crate::malloc::context (内部模块, ctx 全局状态)
  │     └── struct MallocContext { ... }
  │           // 全局分配器上下文 (原 struct malloc_context)
  │           // 关键字段: secret: u64, active: [*mut Meta; 48],
  │           //            usage_by_class: [usize; 48], ...
  │           // 定义于 malloc 模块, 包含全局唯一实例 CTX (静态变量)
  │           // realloc 通过 get_meta 间接依赖 ctx.secret (用于 meta_area 校验)
  │           // rusl 重构: 全局状态使用 static mut 或 Once/Mutex 封装
  │
  ├── crate::platform::mremap (内部平台抽象模块)
  │     └── unsafe fn mremap(
  │             old_addr: *mut c_void,
  │             old_size: usize,
  │             new_size: usize,
  │             flags: i32,
  │         ) -> *mut c_void;
  │           // Linux mremap 系统调用的 rusl 封装
  │           // 通过 asm! 内联汇编直接发起 SYS_mremap
  │           // 前置: old_addr 为有效 mmap 映射地址, old_size/new_size 页对齐
  │           // 后置 Case 1 (成功): 返回新映射区域的起始地址
  │           // 后置 Case 2 (失败): 返回 MAP_FAILED (即 -1_isize as *mut c_void)
  │           // flags: MREMAP_MAYMOVE (允许内核移动映射到新地址)
  │           // rusl 实现: 禁止使用 libc crate, 直接通过 asm!("syscall" ...) 发起
  │
  ├── core 库依赖
  │     ├── core::ptr::copy_nonoverlapping(src: *const T, dst: *mut T, count: usize);
  │     │     // Rust 等价于 C 的 memcpy
  │     │     // 在 Case 5 中将旧数据从 p 拷贝到 new
  │     │     // 前置: src 和 dst 必须对齐, 内存区域不能重叠, count <= 旧/新大小
  │     │     // 注意: 使用 ptr::copy (允许重叠) 或 ptr::copy_nonoverlapping (不允许重叠)
  │     │     //       由于 realloc 中新旧指针指向不同内存块, copy_nonoverlapping 更优
  │     │
  │     ├── core::ffi::c_void   (等价于 C 的 void)
  │     ├── core::usize::MAX    (等价于 C 的 SIZE_MAX)
  │     ├── core::cmp::min      (等价于 C 的 min 宏, 用于取 min(旧大小, n))
  │     └── core::sync::atomic::{AtomicI32, Ordering};
  │           // 用于 meta.avail_mask / meta.freed_mask 的原子操作
  │           // rusl no_std 兼容: AtomicI32 是 core 内建类型
  │
  ├── 外部常量 / 错误机制
  │     ├── errno 全局错误码机制 (thread-local, rusl 自实现)
  │     │     // 前置: errno 存储为线程局部变量, 由 rusl 平台层提供
  │     │     // 后置: 函数返回前设置 errno (ENOMEM 等)
  │     │
  │     ├── ENOMEM: i32   // POSIX ENOMEM 错误码, rusl 自行定义
  │     │
  │     ├── MAP_FAILED: *mut c_void  (= -1_isize as *mut c_void)
  │     │     // mmap/mremap 失败时的返回值
  │     │     // rusl: 直接使用常量, 不依赖 libc crate
  │     │
  │     └── MREMAP_MAYMOVE: i32  // mremap 标志: 允许内核移动映射到新地址
  │           // rusl: 取 Linux 内核定义值 (通常为 1), 自行定义常量
  │
  └── 递归依赖终止
        ├── malloc() — 来自 crate::malloc, 其规约在 malloc.md Rust spec 中独立描述
        ├── free() — 来自 crate::malloc, 其规约在 free.md Rust spec 中独立描述
        ├── memcpy — 由 core::ptr::copy_nonoverlapping 替代, 不依赖外部 libc
        ├── mremap — 由 crate::platform::mremap 封装, 通过 asm! 直接发起系统调用
        ├── errno / ENOMEM / MAP_FAILED / MREMAP_MAYMOVE — rusl 自实现的平台常量
        ├── get_meta / get_slot_index / get_stride / get_nominal_size / set_size / size_to_class / size_overflows — meta 模块内部函数
        ├── Meta / Group / UNIT / IB / MMAP_THRESHOLD — meta 模块内部类型和常量
        ├── SIZE_CLASSES — malloc 模块内部静态数组
        └── ctx — malloc 模块的全局分配器上下文 (通过 get_meta 间接依赖 ctx.secret)
```

---

## [GUARANTEE]

### 对外导出接口

```rust
// [Visibility]: Public — POSIX.1-2001 / ISO C89 标准函数, <stdlib.h> 声明
// [ABI Compatibility]: extern "C", 参数布局与原 C 接口完全兼容
// [符号名]: 直接以 "realloc" 导出, 不经过宏重命名
#[no_mangle]
pub unsafe extern "C" fn realloc(p: *mut core::ffi::c_void, n: usize) -> *mut core::ffi::c_void;
```

**意向 (Intent)**: 更改 `p` 指向的内存块大小为 `n` 字节。采用多级策略，按优先级递减尝试：原地大小调整（最优，零拷贝） → mremap 重映射（mmap 大块场景） → malloc+copy+free（通用回退路径）。尽量减少数据拷贝和系统调用。

---

#### 前置条件

1. **p 为空**: 若 `p.is_null()`，函数等价于 `malloc(n)`。
2. **p 非空**: `p` 必须是先前由 `malloc()`、`calloc()`、`realloc()`、`aligned_alloc()` 或 `posix_memalign()` 返回的有效指针，且尚未被 `free()` 或 `realloc()` 释放。
3. **对齐**: `p` 必须满足 16 字节对齐（`(p as usize) & 15 == 0`），由 `get_meta()` 内部断言保证。
4. **无锁持有要求**: 调用者无需持有任何锁（内部通过 `malloc`/`free` 自行管理锁）。

---

#### 后置条件

##### Case 1: `p.is_null()` (等效于 malloc)

- 直接调用 `malloc(n)` 分配新内存。
- **成功**: 返回分配得到的指针，内存内容未初始化。
- **失败**: 返回 `core::ptr::null_mut()`，`errno = ENOMEM`。

##### Case 2: `n` 导致溢出 (`size_overflows(n) == true`)

- 返回 `core::ptr::null_mut()`，设置 `errno = ENOMEM`。
- 原内存块 `p` 保持有效且未被释放，调用者必须后续显式 `free(p)`。

##### Case 3: 原地缩容/扩容 (最优路径, 零拷贝)

**触发条件** (三个条件同时满足):
1. `n <= avail_size` —— 新大小不超过槽位可用空间
2. `n < MMAP_THRESHOLD` (131052 字节) —— 不触发大块阈值
3. `size_to_class(n) + 1 >= g.sizeclass` —— 新大小类别与原类别相同、相邻或更大

**计算过程**:
```
g = get_meta(p)           // 定位元数据 (含安全断言)
idx = get_slot_index(p)   // 提取槽位索引 (0-31)
stride = get_stride(g)    // 获取槽位跨度
start = g.mem.storage_ptr().add(stride * idx)  // 槽位起始地址
end = start.add(stride - IB)                    // 槽位末尾 (减去 IB 哨兵空间)
avail_size = (end as usize) - (p as usize)      // 从用户指针到末尾的可用字节
```

**动作**: 调用 `set_size(p, end, n)` 就地更新记录的大小。
**返回**: 原指针 `p`（内存地址不变，无数据拷贝）。
**数据完整性**: 原有数据在 `min(旧大小, n)` 范围内保持不变。

##### Case 4: mremap 重映射 (mmap 大块优化路径)

**触发条件** (两个条件同时满足):
1. `g.sizeclass >= 48` —— 原块为大对象（独立 mmap 分配）
2. `n >= MMAP_THRESHOLD` (131052 字节) —— 新大小也达到大块阈值

**前置断言**: `g.sizeclass == 63`（必须为独立 mmap 分配, 非子分配组）
**rusl 实现**: 使用 `debug_assert!` 或 `assert!` (Release 也保留)

**计算过程**:
```
base = (p as usize) - (start as usize)  // 用户数据在 mmap 区域内的偏移量
needed = (n + base + UNIT + IB + 4095) & !4095  // 向上取整到页对齐

// needed 组成:
//   n:     用户请求大小
//   base:  从映射起始到用户数据的偏移 (含 Group 头部和 enframe 偏移)
//   UNIT:  Group 头部大小 (16 字节)
//   IB:    尾部哨兵空间 (4 字节)
//   +4095 & !4095: 向上取整到 4096 字节页边界
```

**子情况 4a: 新大小恰好等于原大小**:
- 若 `g.maplen * 4096 == needed`，无需重新映射。
- 直接复用现有映射，跳过 mremap 系统调用。

**子情况 4b: 需要 mremap**:
- 调用 `mremap(g.mem as *mut c_void, g.maplen * 4096, needed, MREMAP_MAYMOVE)`
- `MREMAP_MAYMOVE` 标志允许内核移动映射到新地址

**成功处理**:
- 若 `new != MAP_FAILED`:
  - 更新元数据: `g.mem = new as *mut Group`, `g.maplen = needed / 4096`
  - 重新计算用户指针: `p = g.mem.storage_ptr().add(base)`
  - 重新计算边界: `end = g.mem.storage_ptr().add(needed - UNIT - IB)`
  - 写入尾部哨兵: `*end = 0`
  - 调用 `set_size(p, end, n)` 更新大小记录
  - 返回更新后的 `p`

**失败处理**:
- 若 `new == MAP_FAILED`，内核保证原映射保持不变。
- **不返回 NULL**，继续执行 Case 5 的 malloc+copy+free 回退路径。
- rusl 实现: 这是关键安全保证, mremap 失败后代码必须继续执行 Case 5, 不能提前返回。

##### Case 5: malloc+copy+free (通用回退路径)

**触发条件**: Case 3 和 Case 4 的条件均不满足，或 Case 4 的 mremap 失败。

**动作**:
1. `new = malloc(n)` —— 分配新内存块
2. 若 `new.is_null()`: 返回 `core::ptr::null_mut()`, `errno = ENOMEM`, 原块 `p` 保持有效
3. `core::ptr::copy_nonoverlapping(p as *const u8, new as *mut u8, min(n, old_size))` —— 拷贝数据
4. `free(p)` —— 释放旧内存块
5. 返回 `new`

**数据完整性**: 原有数据拷贝到新地址（上限为 `min(old_size, n)`），超出部分内容未初始化。
**rusl 重构**: 使用 `core::ptr::copy_nonoverlapping` 替代 C `memcpy`，编译器可内联优化。

---

#### 不变量

1. **Inv 1 (数据安全)**: 在 Case 2 溢出失败和 Case 5 malloc 失败时，原内存块 `p` 始终保持有效且内容不变。调用者必须在失败时继续持有 `p` 并在后续显式 `free(p)`。rusl 实现: 确保所有失败路径在返回 null 前不修改原块。

2. **Inv 2 (原地调整安全性)**: Case 3 的原地缩容/扩容保证了 `n <= end - p`，即新大小不超过槽位物理容量，不会越界写入。

3. **Inv 3 (sizeclass 单调性)**: Case 3 中的条件 `size_to_class(n) + 1 >= g.sizeclass` 确保新大小类别不低于原类别太多。例如原块在类别 10（分配 32-42 单元），缩容到仅需 5 单元时，新类别 5 远小于原类别 10，不满足条件，会走 Case 5 分配更小类别的新块——避免在大槽位中浪费空间。

4. **Inv 4 (mmap 大小一致性)**: 在 Case 4 成功后，`g.maplen` 总是等于 `needed / 4096`，即映射的页数精确对应计算出的页对齐大小。

5. **Inv 5 (哨兵字节)**: 每次成功调用 `set_size` 后，`end[-reserved]`（或 `end[-5]` 当 reserved >= 5 时）和 `*end` 处均有哨兵字节 0，用于 `free` 时的完整性验证和越界写入检测。

6. **Inv 6 (errno 透明性)**: mremap 系统调用可能修改 `errno`。在 mremap 成功路径中，errno 无需特别处理；在 mremap 失败回退到 Case 5 时，后续的 `malloc` 调用会重新设置 `errno`。rusl 实现: 确保不会保留无意义的中间 `errno` 值。

7. **Inv 7 (指针有效性)**: 返回的非空指针必须能被 `get_meta()` 正确解析，即头部字段与组结构一致。该不变量由 `set_size` 的编码规则和 `get_meta` 的校验链保证。

---

#### 系统算法 (System Algorithm)

**Level 1: 元数据定位阶段**

```
// 从用户指针逆向定位所有元数据 (O(1))
g = get_meta(p)                                    // 定位 struct meta
idx = get_slot_index(p)                             // 提取槽位索引 (0-31)
stride = get_stride(g)                              // 计算槽位跨度
start = g.mem.storage_ptr().add(stride * idx)       // 槽位起始地址
end = start.add(stride - IB)                        // 槽位末尾 (减去 4 字节哨兵)
old_size = get_nominal_size(p, end)                 // 解码原始分配大小
avail_size = (end as usize) - (p as usize)          // 当前可用空间大小
```

**Level 2: 三路策略选择**

```
if n <= avail_size && n < MMAP_THRESHOLD && size_to_class(n) + 1 >= g.sizeclass:
    → PATH A: 原地更新 (set_size) → return p              // 最优路径, 零拷贝

if g.sizeclass >= 48 && n >= MMAP_THRESHOLD:
    assert(g.sizeclass == 63)
    → PATH B: mremap 重映射
    if mremap 成功:
        → 更新 g.mem, g.maplen, set_size → return p      // 无数据拷贝
    // mremap 失败时, 内核保证原映射不变, 继续进入 PATH C

→ PATH C: malloc + copy_nonoverlapping + free → return new  // 通用回退
```

**PATH C 的正确性论证**: 即使在 PATH B (mremap) 失败后也走 PATH C。因为 mremap 失败时 Linux 内核保证原映射保持不变，`p` 仍然指向有效内存。此时代码直接进入 `new = malloc(n)`，若成功则 `copy_nonoverlapping` + `free(p)`，若失败则返回 NULL 且 `p` 仍然有效。

---

#### 线程安全性

通过内部 `malloc`/`free` 的锁机制保证线程安全。`realloc` 自身不直接获取锁，而是依赖被调用的 `malloc`/`free` 内部加锁。

rusl 实现: `realloc` 函数体不加锁，锁操作由 `malloc()` 和 `free()` 调用内部处理。

---

#### 信号安全性

不是 async-signal-safe。持有锁期间被信号中断可能导致死锁。

---

### 内部实现函数

#### `realloc_impl` (私有函数)

```rust
// [Visibility]: Internal — rusl 内部实现函数, 不对外导出
//   被 pub unsafe extern "C" fn realloc 直接调用 (或内联于 realloc 体内)
//   封装核心逻辑, 与 C 侧 __libc_realloc 行为等价
unsafe fn realloc_impl(p: *mut c_void, n: usize) -> *mut c_void;
```

**前置条件**:
- 同 `realloc` 的前置条件
- 调用者确保 `p` 和 `n` 参数有效

**后置条件**:
- 与 `realloc` 的 Case 2/3/4/5 后置条件完全相同

**rusl 实现策略**: 可以直接内联为 `realloc` 函数体的私有辅助段（例如 `if p.is_null() { return malloc(n) }` 后进入主逻辑）。由于 Rust 的 `unsafe` 块可以很好地组织，不一定需要单独的函数。如果选择分离为 `realloc_impl`，则使用 `pub(crate)` 或更小可见性。

---

## 内部依赖符号汇总

| 符号 | Rust 类型/表示 | 来源模块 | 可见性 |
|------|---------------|---------|--------|
| `realloc` | `pub unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void` | realloc 模块 | **Public** `<stdlib.h>` |
| `realloc_impl` | `unsafe fn(*mut c_void, usize) -> *mut c_void` | realloc 模块 | Internal `pub(crate)` |
| `malloc` | `pub unsafe extern "C" fn(usize) -> *mut c_void` | malloc 模块 | **Public** `<stdlib.h>` |
| `free` | `pub unsafe extern "C" fn(*mut c_void)` | free 模块 | **Public** `<stdlib.h>` |
| `mremap` | `unsafe fn(*mut c_void, usize, usize, i32) -> *mut c_void` | platform 模块 | Internal |
| `errno` | 全局可写变量 (thread-local) | 错误处理模块 | **Public** `<errno.h>` |
| `ENOMEM` | 常量 `i32` | 错误处理模块 | **Public** `<errno.h>` |
| `MAP_FAILED` | 常量 `*mut c_void` (= -1_isize as *mut c_void) | platform 模块 | Internal |
| `MREMAP_MAYMOVE` | 常量 `i32` | platform 模块 | Internal |
| `SIZE_MAX` | `core::usize::MAX` | Rust 语言内建 | Public |
| `UNIT` | `const usize = 16` | meta 模块 | Internal |
| `IB` | `const usize = 4` | meta 模块 | Internal |
| `MMAP_THRESHOLD` | `const usize = 131052` | meta 模块 | Internal |
| `Meta` | `struct Meta` (Repr(C)) | meta 模块 | Internal |
| `Group` | `struct Group` (Repr(C)) | meta 模块 | Internal |
| `SIZE_CLASSES` | `static [u16; 48]` | malloc 模块 | Internal |
| `size_overflows` | `fn(usize) -> bool` | meta 模块 | Internal |
| `get_meta` | `unsafe fn(*const u8) -> &Meta` | meta 模块 | Internal |
| `get_slot_index` | `fn(*const u8) -> usize` | meta 模块 | Internal |
| `get_stride` | `fn(&Meta) -> usize` | meta 模块 | Internal |
| `get_nominal_size` | `unsafe fn(*const u8, *const u8) -> usize` | meta 模块 | Internal |
| `set_size` | `unsafe fn(*mut u8, *mut u8, usize)` | meta 模块 | Internal |
| `size_to_class` | `fn(usize) -> usize` | meta 模块 | Internal |
| `core::ptr::copy_nonoverlapping` | Rust 标准库 (core) | `core::ptr` | Public |

---

## 跨文件依赖说明

| 依赖符号 | 来源文件 (Rust) | 来源文件 (C) | 说明 |
|---------|---------|---------|------|
| `malloc()` | `malloc.rs` (mallocng) | `malloc.c` (mallocng) | Case 1 (p 为空) 和 Case 5 中分配新内存块 |
| `free()` | `free.rs` (mallocng) | `free.c` (mallocng) | Case 5 中释放旧内存块 |
| `copy_nonoverlapping` | `core::ptr` | `<string.h>` (memcpy) | Case 5 中拷贝数据, Rust 零成本替代 |
| `mremap()` | `platform/syscall.rs` | `glue.h` → `__mremap` | Case 4 中重映射 mmap 区域, rusl 通过 asm! 实现 |
| `Meta` / `Group` | `meta.rs` (mallocng) | `meta.h` | 核心数据结构和常量 |
| `UNIT` / `IB` / `MMAP_THRESHOLD` | `meta.rs` (mallocng) | `meta.h` | 内部常量 |
| `SIZE_CLASSES[]` | `malloc.rs` (mallocng) | `malloc.c` (mallocng) | 大小类别查找表 |
| `size_overflows()` | `meta.rs` (mallocng) | `meta.h` (inline) | 溢出检查 |
| `get_meta()` / `get_slot_index()` / `get_stride()` / `get_nominal_size()` / `set_size()` / `size_to_class()` | `meta.rs` (mallocng) | `meta.h` (inline) | 核心内联辅助函数 |
| `errno` / `ENOMEM` | 错误处理模块 | `<errno.h>` | POSIX 错误码机制, rusl 自实现 |
| `MAP_FAILED` / `MREMAP_MAYMOVE` | platform 模块 | `<sys/mman.h>` | syscall 相关常量, rusl 自定义 |

---

## rusl no_std 适配说明

1. **无 `libc` crate**: 所有 C ABI 类型使用 `core::ffi::c_void`、`usize`（等价 `size_t`）、`i32`（等价 `c_int`）、`*const u8` / `*mut u8`（等价 `unsigned char *`）等 Rust 原生类型。

2. **`#![no_std]` 约束**: 不依赖 `std::alloc`，内部使用 mallocng 自己的分配器；`core::ptr::null_mut()` 替代 `std::ptr::null_mut()`；`core::ptr::copy_nonoverlapping` 替代 `memcpy`。

3. **`memcpy` 替代**: C 侧 Case 5 中调用外部 `memcpy`，rusl 使用 `core::ptr::copy_nonoverlapping`（或 `core::ptr::copy`）。由于新旧指针指向不同内存块且无重叠，`copy_nonoverlapping` 更优。编译器可将此内联为高效的 SIMD 拷贝指令。

4. **`mremap` 系统调用**: musl C 侧通过 `glue.h` 宏 `#define mremap __mremap` 映射到 musl 内部 syscall 封装。rusl 必须通过 `asm!` 内联汇编直接发起 `SYS_mremap` 系统调用，不得经过任何外部 libc FFI。封装为 `crate::platform::mremap` 内部函数。

5. **`errno` 机制**: rusl 需自行实现 thread-local `errno` 存储及 `ENOMEM` 常量定义，不依赖外部 `libc` 的 `__errno_location`。`size_overflows` 函数中设置 `errno = ENOMEM` 需要通过 rusl 自己的 errno 写入接口。

6. **`MAP_FAILED` / `MREMAP_MAYMOVE`**: musl C 侧从 `<sys/mman.h>` 获取这些常量。rusl 需在自己的 platform 模块中定义这些常量值（均为 Linux 内核 ABI 固定值，不随发行版变化）。

7. **位域结构体**: `Meta` 和 `Group` 结构体使用 C 位域（`:5`, `:6` 等）。rusl 使用 `#[repr(C)]` 保证内存布局与 C 一致，位域的读写需通过手动掩码/位移操作实现（或使用 `bitfield` 等支持 `no_std` 的第三方 crate）。关键要求：`Meta` 的总布局（特别是 `maplen` 字段的偏移量和 `last_idx:5`/`freeable:1`/`sizeclass:6` 的位打包）必须与 C 侧完全相同，以保证 `get_meta()` 中的指针运算正确。

8. **原子操作**: C 侧 `Meta` 的 `avail_mask` 和 `freed_mask` 声明为 `volatile int` 并使用 `a_cas` / `a_or` 原子操作。rusl 使用 `core::sync::atomic::AtomicI32` + `Ordering` 参数替代，消除对 `volatile` 和外部原子库的依赖。

9. **断言机制**: C 侧使用 `glue.h` 的 `assert(x)` 宏（默认调用 `a_crash()`）。rusl 内部对安全关键的断言使用 `assert!`（Release 模式保留），对性能敏感的调试断言使用 `debug_assert!`（Release 模式移除）。`get_meta()` 中的校验链属于安全关键，必须保留在 Release 构建中。

10. **全局 ctx 上下文**: C 侧 `ctx` 是全局变量，通过 `glue.h` 宏 `#define ctx __malloc_context` 访问。rusl 中 `ctx` 可以设计为模块内的 `static` 变量，通过 `meta` 模块的接口函数访问（例如 `meta::secret()` 返回 `ctx.secret`）。`realloc` 不直接访问 `ctx`，仅通过 `get_meta()` 间接依赖 `ctx.secret`（用于 `meta_area.check` 校验）。

---

## 安全考虑 (rusl 重构)

1. **整数溢出防护**: 函数入口立即检查 `size_overflows(n)`，防止 `n` 过大导致后续计算（如 `needed = (n + base + UNIT + IB + 4095) & !4095`）溢出。rusl 中 `size_overflows` 使用 `usize::checked_add` 等安全方法进行内部计算。

2. **unsafe 块最小化**: rusl 内部实现中，unsafe 操作集中封装在 `meta` 模块的辅助函数中（`get_meta`、`set_size`、`get_nominal_size` 等）。`realloc` 函数体的 unsafe 范围仅限于：
   - 调用 `get_meta(p)` 获取元数据引用
   - 调用 `set_size(p, end, n)` 写入头部
   - 原始指针算术（`start.add(...)`, `end` 计算）
   - 调用 `mremap` / `malloc` / `free` / `copy_nonoverlapping`

3. **元数据 corruption 检测**: `get_meta(p)` 内含多重断言：
   - `meta_area.check == ctx.secret` —— 防止伪造/损坏的 meta area
   - 偏移量范围检查 —— 确保指针确实指向有效槽位
   - `avail_mask`/`freed_mask` 检查 —— 确保槽位当前处于"已分配"状态
   - rusl 中这些断言使用 `assert!` 宏（Release 构建保留），失败时调用 `panic!`（等价于 `a_crash`）

4. **mremap 失败安全**: 当 `mremap` 返回 `MAP_FAILED` 时，Linux 内核保证原映射保持不变。rusl 实现必须确保在此情况下不回退到返回 NULL，而是优雅地继续执行 Case 5 的 malloc+copy+free。

5. **copy_nonoverlapping 安全**: Case 5 中 `copy_nonoverlapping(p, new, min(n, old_size))` 的前置条件是 `p` 和 `new` 指向不同且不重叠的内存区域。realloc 场景中，`p` 属于旧块而 `new` 来自全新的 `malloc(n)` 分配，二者必定不重叠，使用 `copy_nonoverlapping` 安全且更高效。

6. **失败不泄漏**: 若 PATH C 中 `malloc(n)` 成功但后续 `copy_nonoverlapping` 或 `free` 触发段错误（如 `p` 已损坏），则新块 `new` 会泄漏。这是 realloc 语义的内在限制——一旦进入"分配新块"阶段，无法再安全回退到原块。rusl 实现中无需额外处理此边界情况，因为其对应的是内存损坏 bug 而非正常执行路径。