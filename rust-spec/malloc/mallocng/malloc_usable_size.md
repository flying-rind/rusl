# malloc_usable_size Rust 接口

## 复杂度分级: Level 2

> C 源文件: `src/malloc/mallocng/malloc_usable_size.c`
> 对应 C spec: `src/malloc/mallocng/spec/malloc_usable_size.md`

---

## [RELY]

```
malloc_usable_size (对外导出, extern "C")
  │
  ├── crate::malloc::meta (内部模块, 重新设计)
  │     │  // 核心数据结构 (原 meta.h 中的 struct meta / struct group / struct meta_area)
  │     │
  │     ├── struct Meta { ... }
  │     │     // 元数据结构体 (原 struct meta)
  │     │     // 字段: prev, next (双向循环链表指针, *mut Meta / *const Meta)
  │     │     //       mem (*mut Group, 指向关联的 Group)
  │     │     //       avail_mask (AtomicI32, 可用槽位位掩码, C 侧 volatile int)
  │     │     //       freed_mask (AtomicI32, 已释放待回收槽位位掩码)
  │     │     //       last_idx: u8 (5-bit 位域, 槽位最大索引 0..31)
  │     │     //       freeable: bool (1-bit 位域, 该组是否可整体释放)
  │     │     //       sizeclass: u8 (6-bit 位域, 0..47 常规类 / 63 表示 mmap)
  │     │     //       maplen: usize (mmap 分配时的页数, 非 mmap 时为 0)
  │     │
  │     ├── struct Group { ... }
  │     │     // 内存组结构体 (原 struct group)
  │     │     // 字段: meta (*const Meta, 指向所属 Meta)
  │     │     //       active_idx: u8 (5-bit 位域, 当前活跃最大 slot 索引)
  │     │     //       pad: [u8; UNIT - size_of::<*const Meta>() - 1]
  │     │     //       storage: [u8; 0] (柔性数组, 实际存储区, 用 *mut u8 间接访问)
  │     │     // 不变量: Group 起始地址页对齐; &group == group.meta.mem
  │     │
  │     ├── struct MetaArea { ... }
  │     │     // 元数据区域结构体 (原 struct meta_area)
  │     │     // 字段: check (u64, 安全校验值, 须等于 ctx.secret)
  │     │     //       next (*mut MetaArea, 链表指针)
  │     │     //       nslots (i32, 槽位数量)
  │     │     //       slots: [Meta; 0] (柔性数组)
  │     │     // 不变量: (area_ptr & (4096-1)) == 0 (页对齐)
  │     │
  │     ├── struct MallocContext { ... }
  │     │     // 全局分配上下文 (原 struct malloc_context)
  │     │     // 字段: secret (u64, 随机密钥)
  │     │     //       pagesize (usize, 仅在编译时未定义 PAGESIZE 时存在)
  │     │     //       init_done (bool / AtomicBool)
  │     │     //       mmap_counter (u32)
  │     │     //       free_meta_head (*mut Meta, 空闲 meta 双向循环链表)
  │     │     //       avail_meta / avail_meta_count / avail_meta_area_count
  │     │     //       meta_alloc_shift (usize)
  │     │     //       meta_area_head / meta_area_tail (*mut MetaArea)
  │     │     //       avail_meta_areas (*mut u8, 位图)
  │     │     //       active ([*mut Meta; 48], 48 个 sizeclass 的活跃链表头)
  │     │     //       usage_by_class ([usize; 48])
  │     │     //       unmap_seq ([u8; 32]), bounces ([u8; 32])
  │     │     //       seq (u8, 全局操作序列号 1-255)
  │     │     //       brk (usize / *mut u8)
  │     │
  │     ├── pub(crate) static CTX: MallocContext;
  │     │     // 全局唯一分配上下文实例 (原 extern struct malloc_context ctx)
  │     │     // 通过 glue.h 重映射为 __malloc_context
  │     │     // 多线程安全: 修改须持有锁, 纯读操作可利用 Atomic 类型直接访问
  │     │
  │     ├── pub(crate) const UNIT: usize = 16;
  │     │     // 基本分配单元 / 最小对齐粒度 (原 #define UNIT 16)
  │     │     // 所有分配大小向上取整到 UNIT 倍数
  │     │
  │     ├── pub(crate) const IB: usize = 4;
  │     │     // In-band header 大小 (原 #define IB 4)
  │     │     // 每个 slot 末尾预留的越界检测字节数
  │     │     // 不变量: UNIT >= IB 始终成立
  │     │
  │     ├── pub(crate) static SIZE_CLASSES: [u16; 48];
  │     │     // 48 个大小类别的槽位容量表 (以 UNIT 为单位)
  │     │     // 定义于 malloc 模块, 等价于原 size_classes[]
  │     │     // 对于 sizeclass 0..9: SIZE_CLASSES[i] 精确对应 i 个 UNIT
  │     │     // 对于更大 class: 表示该 class 的最小容量
  │     │
  │     ├── pub(crate) unsafe fn get_slot_index(p: *const u8) -> usize;
  │     │     // 从分配指针的 in-band header 中提取槽位索引
  │     │     // 前置条件: p 非空, 指向 mallocng 分配的有效用户指针
  │     │     // 后置条件: 返回 (p[-3] & 31) 即 header 字节低 5 位 (0..31)
  │     │     // 等价于原 static inline int get_slot_index(const unsigned char *p)
  │     │
  │     ├── pub(crate) unsafe fn get_meta(p: *const u8) -> &Meta;
  │     │     // 从用户指针逆向推导对应的 Meta 控制块 (核心安全校验函数)
  │     │     // 前置条件: p 非空, (p as usize) & 15 == 0 (16 字节对齐)
  │     │     //          p 指向 mallocng 分配的有效用户指针
  │     │     // 后置条件 (成功): 返回 p 所属 Group 的 Meta 引用
  │     │     // 后置条件 (失败): 任一断言失败则调用 abort() 终止进程
  │     │     // 校验链 (多层叠加, 与原 C 版对应):
  │     │     //   1. assert!((p as usize) & 15 == 0)                          — 地址 16 字节对齐
  │     │     //   2. offset = read_u16(p[-2..]) as usize; idx = get_slot_index(p)
  │     │     //   3. 若 p[-4] != 0 (非零偏移 enframe): 偏移量实际存储于 p[-8] 的 u32;
  │     │     //      assert!(offset > 0xFFFF)
  │     │     //   4. base = p - UNIT * offset - UNIT  (逆推 Group 基址)
  │     │     //   5. meta = &*base.meta  (原始设计中 base 为 *mut Group)
  │     │     //   6. assert!(meta.mem as usize == base as usize)              — 双向绑定验证
  │     │     //   7. assert!(idx <= meta.last_idx as usize)                   — 索引不越界
  │     │     //   8. assert!(meta.avail_mask & (1 << idx) == 0)                — 槽位确已分配
  │     │     //   9. assert!(meta.freed_mask & (1 << idx) == 0)                — 槽位未被释放
  │     │     //  10. area = (meta as *const Meta as usize & !4095) as *const MetaArea
  │     │     //      assert!(area.check == CTX.secret)                        — 密钥防伪造
  │     │     //  11. 若 sizeclass < 48: 验证偏移量与 sizeclass 一致性
  │     │     //  12. 若 sizeclass == 63: assert!(meta.sizeclass == 63)        — mmap 确认
  │     │     //  13. 若 meta.maplen > 0: assert!(offset < 范围)
  │     │     // 等价于原 static inline struct meta *get_meta(const unsigned char *p)
  │     │     // 设计考量: 返回 &Meta 而非 *const Meta, 因为调用上下文 (malloc_usable_size)
  │     │     //          只执行纯读操作, 不修改 Meta; 不可变引用比裸指针更安全
  │     │
  │     ├── pub(crate) fn get_stride(m: &Meta) -> usize;
  │     │     // 返回给定 Meta 所描述 Group 中每个 slot 的跨步大小
  │     │     // 前置条件: m 有效 (由 get_meta 返回)
  │     │     // 后置条件:
  │     │     //   Case 1 (mmap 大块, last_idx == 0 && maplen > 0):
  │     │     //     stride = maplen * PGSZ - UNIT  (整块 mmap 区减去 Group 头部)
  │     │     //   Case 2 (常规 slab 组):
  │     │     //     stride = UNIT * SIZE_CLASSES[m.sizeclass as usize]
  │     │     // 等价于原 static inline size_t get_stride(const struct meta *g)
  │     │
  │     └── pub(crate) unsafe fn get_nominal_size(p: *const u8, end: *const u8) -> usize;
  │           // 从分配块的 header 中解码用户可用字节数
  │           // 前置条件: p 指向分配块起始; end = start + stride - IB (slot 末尾)
  │           // 编码规则:
  │           //   reserved = p[-3] >> 5  (高 3 位, 值域 0..7)
  │           //   若 reserved < 5: 保留大小为 reserved 字节
  │           //   若 reserved >= 5 (实际为 5): 保留大小溢出存储, 从 end[-4] 读 u32
  │           //     assert!(end[-5] == 0)  (溢出检测标记)
  │           //   计算: end - reserved - p  (slot 可用空间减去保留区)
  │           // 后置条件: 返回值 ∈ [0, stride - IB]
  │           //   assert!(reserved <= end - p)  — 保留大小不超 slot 实际空间
  │           //   assert!(*(end - reserved) == 0)  — 分隔零字节
  │           // 等价于原 static inline size_t get_nominal_size(const unsigned char *p, const unsigned char *end)
  │
  ├── crate::malloc::glue (内部模块)
  │     │  // 原 glue.h 中的命名空间重映射在 rusl 中不再需要
  │     │  // (Rust 模块系统天然提供命名空间隔离)
  │     │  // 但锁原语/系统调用封装仍需保留
  │     │
  │     ├── const PGSZ: usize;               // 页大小 (编译期常量或 CTX.pagesize)
  │     ├── fn rdlock();                      // 读锁 (RDLOCK_IS_EXCLUSIVE=1 时等价排他锁)
  │     ├── fn wrlock();                      // 写锁
  │     ├── fn unlock();                      // 释放锁
  │     ├── fn assert_or_crash(cond: bool);   // 断言失败时调用 a_crash 终止进程
  │     │     // 等价于原 assert(x) → { if (!(x)) a_crash(); }
  │     │     // 注意: malloc_usable_size 自身不获取锁也不调用此函数;
  │     │     //       该依赖仅通过 get_meta 内部的 assert 链间接引入
  │     └── fn get_random_secret() -> u64;    // 生成随机密钥 (用于 meta_area.check)
  │
  ├── crate::syscall (内部模块, 重新设计)
  │     │  // 原 glue.h 中通过 <sys/mman.h> / <unistd.h> 引用的系统调用封装
  │     │  // rusl 通过 asm! 内联汇编直接发起 syscall, 不经过 libc crate
  │     │
  │     ├── unsafe fn brk(addr: usize) -> usize;          // SYS_brk
  │     ├── unsafe fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;  // SYS_mmap
  │     ├── unsafe fn madvise(addr: *mut u8, len: usize, advice: i32) -> i32;  // SYS_madvise
  │     └── unsafe fn munmap(addr: *mut u8, len: usize) -> i32;                // SYS_munmap
  │
  ├── 外部常量 / 错误机制
  │     ├── core::ffi::c_void     (等价于 C void)
  │     ├── usize                 (等价于 C size_t)
  │     └── u8 / u16 / u32 / u64  (等价于 C uintN_t)
  │
  └── 递归依赖终止
        ├── Meta / Group / MetaArea / MallocContext / CTX — meta 模块内部类型, 规约见 meta.md 的 Rust spec
        ├── get_meta / get_slot_index / get_stride / get_nominal_size — meta 模块内部函数
        ├── UNIT / IB / SIZE_CLASSES — meta 模块内部常量和静态数据
        ├── rdlock / wrlock / unlock / assert_or_crash — glue 模块函数
        ├── get_random_secret / PGSZ — glue 模块
        ├── brk / mmap / madvise / munmap — syscall 模块 (rusl 通过 asm! 自行封装)
        ├── a_crash — 进程终止原语 (等价于 core::intrinsics::abort 或非法指令)
        ├── abort() — 来自 core::intrinsics::abort (rusl #![no_std] 下等价)
        └── errno / ENOMEM — C 标准库全局 errno 机制, 外部模块
           (malloc_usable_size 实际不设置 errno, 仅依赖 get_meta 的 assert 路径)
```

---

## [GUARANTEE]

### 对外导出接口

```rust
// [Visibility]: Public — GNU 扩展 API, <malloc.h> 声明
// [ABI Compatibility]: extern "C", 参数布局与原 C 接口完全兼容
//                      size_t → usize, void *p → *mut c_void
#[no_mangle]
pub unsafe extern "C" fn malloc_usable_size(p: *mut core::ffi::c_void) -> usize;
```

#### 前置条件

1. **指针来源**: `p` 必须是以下之一：
   - 由 `malloc()` / `calloc()` / `realloc()` 返回的有效指针 (且未被 `free()` 释放)
   - `core::ptr::null_mut()` (NULL 指针, GNU 扩展约定)
2. **并发约束**:
   - 本函数不获取任何锁 (`rdlock` / `wrlock`), 仅执行带内元数据纯读操作
   - 若 `p` 被另一线程并发 `free()` 或 `realloc()`, 行为未定义 (use-after-free)
   - 调用者需确保在并发环境下正确同步
3. **地址对齐**: 若 `p` 非 NULL, 则 `(p as usize) & 15 == 0` 必须成立
   (get_meta 内部会通过 assert 检查此条件)

#### 后置条件

**Case 1 (p == NULL)**: 返回 `0`。GNU 扩展约定行为。

**Case 2 (p != NULL, 有效指针)**: 返回 `p` 所指向内存块的实际可用字节数。

返回值 >= 原始请求大小 (因为大小类别取整可能导致实际分配大于请求)。
上界为当前 slot 的 `stride - IB - reserved` (任何保留字节)。

**计算过程** (纯读操作, O(1)):
1. 通过 `get_meta(p)` 定位所属 Meta 控制块
2. 通过 `get_slot_index(p)` 获取 slot 索引 `idx`
3. 通过 `get_stride(&meta)` 获取 slot 跨步大小 `stride`
4. 计算 slot 起始地址: `start = meta.mem.storage_ptr().add(stride * idx)`
5. 计算有效区域末尾: `end = start.add(stride - IB)`
6. 通过 `get_nominal_size(p, end)` 从 in-band header 解码可用大小

**简化后的 Rust 伪代码**:
```rust
pub unsafe extern "C" fn malloc_usable_size(p: *mut c_void) -> usize {
    if p.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees p is valid allocated pointer from mallocng
    let p = p as *const u8;
    let meta = get_meta(p);                         // 多层 assert 校验
    let idx = get_slot_index(p);                     // p[-3] & 31
    let stride = get_stride(meta);                   // UNIT * SIZE_CLASSES[sc] 或 maplen * PGSZ - UNIT
    let start = (meta.mem as *const u8).add(UNIT)    // Group::storage 起始
                    .add(stride * idx);              // 定位到具体 slot
    let end = start.add(stride - IB);                // slot 末尾减去 in-band 元数据
    get_nominal_size(p, end)                         // 解码用户可用字节数
}
```

#### 不变量

- **INV-SIZE-LOWER-BOUND**: 对于通过 `malloc(n)` 分配的指针 `p`, 有 `malloc_usable_size(p) >= n`
- **INV-REALLOC-BOUND**: 对于通过 `realloc(p, n)` 分配的指针 `p`, 有 `malloc_usable_size(p) >= n`
- **INV-CALLOC-BOUND**: 对于通过 `calloc(nmemb, size)` 分配的指针 `p`, 有 `malloc_usable_size(p) >= nmemb * size`
- **INV-NO-LOCK**: 本函数不获取也不释放任何锁; 对 `CTX`、`avail_mask`、`freed_mask` 的访问均为纯读, 依赖 Rust 的 `AtomicI32` 提供原子加载语义 (等价于 C `volatile int`)
- **INV-READ-ONLY**: 本函数不修改任何全局状态——不写 `avail_mask`/`freed_mask`, 不改 `CTX` 任何字段

#### 系统算法 (Level 2)

本函数的实现策略是全读操作，通过 in-band header 中的自描述信息在 O(1) 时间内计算出可用大小，无需遍历任何全局数据结构:

**阶段 1 -- NULL 处理**:
```
if p.is_null() => return 0
```
GNU 扩展保证 `malloc_usable_size(NULL)` 返回 0 而非崩溃。

**阶段 2 -- 逆推 Meta 控制块** (`get_meta`):
```
从 p 的 in-band header 中读取偏移量 → 逆推 Group 基址 → 获取 Meta 指针
```
这是 mallocng 的核心设计: 通过在分配指针前嵌入自描述元数据 (偏移量 + slot 索引), 实现 O(1) 反向查找, 无需维护全局指针→元数据映射表。

get_meta 内部执行 13 步 assert 校验链 (见 `[RELY]` 中 get_meta 的说明):
- 地址对齐校验
- 非零偏移 enframe 检测 (double-free 攻击检测)
- Group 基址逆推
- 双向绑定验证 (`meta.mem == base`)
- 槽位状态验证 (`avail_mask` / `freed_mask`)
- meta_area 密钥验证 (`area.check == CTX.secret`)
- sizeclass / maplen 一致性验证

任一 assert 失败 → `abort()`, 防止内存损坏传播。

**阶段 3 -- 计算 slot 跨步** (`get_stride`):
```
Case 1 (mmap): stride = meta.maplen * PGSZ - UNIT
Case 2 (常规): stride = UNIT * SIZE_CLASSES[meta.sizeclass]
```
`get_stride` 统一了常规 slab 和 mmap 大块两种分配模式, 上层代码无需分支处理。

**阶段 4 -- 定位 slot 边界**:
```
start = Group::storage + stride * idx
end   = start + stride - IB
```
每个 slot 的总存储空间为 `stride` 字节, 其中末尾 `IB` (= 4) 字节用于越界检测标记 (overflow check byte), 不计入用户可用空间。

**阶段 5 -- 解码可用大小** (`get_nominal_size`):
```
reserved = (p[-3] >> 5) as usize     // 高 3 位: 0..4 内联, 5 表示溢出存储
if reserved >= 5 {
    reserved = read_u32(end[-4..]);  // 从 slot 末尾读取实际保留值
    assert!(end[-5] == 0);           // 溢出检测标记
}
usable = (end as usize) - reserved - (p as usize)
assert!(reserved <= end - p);     // 保留大小不越界
assert!(*end.sub(reserved) == 0); // 分隔零字节
return usable;
```
mallocng 支持将 slot 的部分空间保留不分配给用户 (预留区 reserved)。reserved < 5 时内联在 `p[-3]` 高 3 位; >= 5 时使用 slot 末尾的扩展存储。用户可用大小 = `slot 总空间 - IB - reserved`。

#### 复杂度

- **时间复杂度**: O(1) -- 所有操作均为常数时间的指针运算和元数据读取
- **空间开销**: 0 -- 无额外分配, 纯读操作
- **线程安全性**: 无锁设计, 依赖 `avail_mask`/`freed_mask` 的原子读取语义

#### 与 GNU/POSIX 标准的关系

`malloc_usable_size` 是 GNU 扩展 (<malloc.h> 声明), POSIX 标准未定义。可移植代码应避免依赖此函数。

**注意事项**:
- 返回值不能用于推断原始请求大小 -- 只能得知分配器实际预留空间
- 多线程并发 free/realloc 同一指针 p 导致 use-after-free 时行为未定义
- Release 构建下 assert 可能被移除 (取决于 rusl 是否使用 `debug_assert!` 或保持 `assert!`)

---

## 内部依赖符号汇总

| 符号 | Rust 类型/表示 | 来源模块 | 可见性 |
|------|---------------|---------|--------|
| `malloc_usable_size` | `extern "C" fn(*mut c_void) -> usize` | malloc_usable_size 模块 | **Public** `<malloc.h>` |
| `Meta` | `struct Meta` | meta 模块 | Internal |
| `Group` | `struct Group` | meta 模块 | Internal |
| `MetaArea` | `struct MetaArea` | meta 模块 | Internal |
| `MallocContext` | `struct MallocContext` | meta 模块 | Internal |
| `CTX` | `pub(crate) static MallocContext` | meta 模块 | Internal |
| `UNIT` | `pub(crate) const usize = 16` | meta 模块 | Internal |
| `IB` | `pub(crate) const usize = 4` | meta 模块 | Internal |
| `SIZE_CLASSES` | `pub(crate) static [u16; 48]` | meta 模块 | Internal |
| `get_meta` | `pub(crate) unsafe fn(*const u8) -> &Meta` | meta 模块 | Internal |
| `get_slot_index` | `pub(crate) unsafe fn(*const u8) -> usize` | meta 模块 | Internal |
| `get_stride` | `pub(crate) fn(&Meta) -> usize` | meta 模块 | Internal |
| `get_nominal_size` | `pub(crate) unsafe fn(*const u8, *const u8) -> usize` | meta 模块 | Internal |
| `PGSZ` | `pub(crate) const usize` 或 `CTX.pagesize` | glue 模块 | Internal |
| `assert_or_crash` | `pub(crate) fn(bool)` | glue 模块 | Internal |
| `brk` / `mmap` / `madvise` / `munmap` | `unsafe fn` (asm! 封装的 syscall) | syscall 模块 | Internal |
| `abort` | `core::intrinsics::abort()` | Rust 语言内建 | Public |

---

## 跨文件依赖说明

| 依赖符号 | 来源文件 | 说明 |
|---------|---------|------|
| `Meta` / `Group` / `MetaArea` / `MallocContext` | `meta.rs` (mallocng) | 核心数据结构和全局上下文 |
| `CTX` | `meta.rs` (mallocng) | 全局唯一分配上下文实例 |
| `UNIT` / `IB` | `meta.rs` (mallocng) | 基本常量和大小宏 |
| `SIZE_CLASSES[]` | `meta.rs` (mallocng) | 48 大小类别查找表 (定义于 malloc.rs) |
| `get_meta()` / `get_slot_index()` / `get_stride()` / `get_nominal_size()` | `meta.rs` (mallocng) | 内部辅助函数 (in-band header 编解码) |
| `PGSZ` | `glue.rs` (mallocng) | 页大小常量或运行时值 |
| `assert_or_crash()` | `glue.rs` (mallocng) | 断言失败 → 进程终止 |
| `get_random_secret()` | `glue.rs` (mallocng) | 随机密钥生成 (用于 meta_area.check) |
| `brk` / `mmap` / `madvise` / `munmap` | `syscall.rs` (rusl 核心) | 系统调用 primitives (asm! 内联汇编) |

---

## 递归依赖终止证明

本 spec 追踪的依赖链在以下节点终止:

1. **Rust 语言内建类型**: `core::ffi::c_void`, `usize`, `u8`, `u16`, `u32`, `u64`, `i32`, `bool` — 无需外部依赖
2. **Rust 内建函数**: `core::intrinsics::abort()` — Rust 编译器提供, 无需外部依赖
3. **rusl 内部模块**: `meta.rs` (本 crate), `glue.rs` (本 crate), `syscall.rs` (本 crate) — 均为 rusl 项目内部实现, 其 spec 各自独立描述
4. **Linux syscall ABI**: `brk` / `mmap` / `madvise` / `munmap` — 直接通过 `asm!` 发起, 不经过任何外部 libc FFI

整体依赖图不依赖任何外部 crate (无 `libc`, 无 `bitflags`, 无 `libm` 等), 完全在 `#![no_std]` 环境内自包含。

---

## rusl no_std 适配说明

1. **无 `libc` crate**: 所有 C ABI 类型使用 `core::ffi::c_void`、`usize` (等价 `size_t`)、`u8`/`u16`/`u32`/`u64` 等 Rust 原生类型
2. **no_std 约束**: 不依赖 `std::alloc`; 函数自身的计算逻辑不使用堆分配; 仅读取已分配内存块的 in-band header
3. **原子操作**: `avail_mask` / `freed_mask` 使用 `core::sync::atomic::AtomicI32` (等价于 C `volatile int` + `a_cas`), 提供原子 load 语义, 无需加锁即可在读路径上安全访问
4. **系统调用**: `brk`/`mmap`/`madvise`/`munmap` 由 rusl 通过 `asm!` 直接发起, 不经过 `libc` crate
   - 注意: `malloc_usable_size` 本身不调用任何系统调用; 这些依赖仅间接通过 `get_meta` → `meta_area` 校验路径引入
5. **命名空间隔离**: Rust 模块系统天然提供命名空间隔离, 无需 `glue.h` 中的 `#define size_classes __malloc_size_classes` 等宏重映射
6. **assert 语义**: 原 C 侧的 `assert(x)` → `{ if (!(x)) a_crash(); }` 在 Rust 中可映射为 `assert!()` (debug 模式) 或保留为始终启用的检查 (release 模式)。鉴于 mallocng 的 assert 用于防堆损坏, 建议 release 构建中也保留核心校验 (使用自定义 `assert_or_crash!`)
7. **`__malloc_replaced` 简化**: rusl 为 `#![no_std]` 静态链接库, 不存在动态链接器替换 `malloc` 的场景, 因此 `is_aligned_alloc_disabled()` 等相关逻辑始终返回 false, 可简化为常量
8. **零开销**: 内部函数的 `#[inline]` 标注确保 get_meta / get_slot_index / get_stride / get_nominal_size 在使用处完全内联展开, 零额外调用开销

---

## 与 C spec 的差异对照

| 项目 | C spec (原) | Rust spec (新) |
|------|-----------|---------------|
| 函数签名 | `size_t malloc_usable_size(void *p)` | `pub unsafe extern "C" fn malloc_usable_size(p: *mut c_void) -> usize` |
| 参数类型 | `void *` (C 指针) | `*mut core::ffi::c_void` (Rust 裸指针) |
| 返回值类型 | `size_t` | `usize` (ABI 兼容) |
| 链接约定 | `extern` (默认) | `extern "C"`, `#[no_mangle]` |
| 内部指针运算 | C 裸指针算术 | Rust `*const u8` / `*mut u8` + `.add()` / `.sub()` |
| 内部 assert | `assert(x)` → `a_crash()` | `assert!(x)` 或自定义 `assert_or_crash!(x)` |
| volatile 字段 | C `volatile int` | Rust `AtomicI32` (提供等价原子语义) |
| 锁语义 | 函数不获取锁 | 同 — 纯读操作, 不获取锁 |
| 内部函数可见性 | `static inline` (翻译单元内) | `pub(crate)` (crate 内可见) |
| 依赖声明 | `glue.h` / `meta.h` header 包含 | `use crate::malloc::meta::*` (Rust 模块系统) |
| 系统调用封装 | `__syscall(SYS_brk, ...)` 宏 | `asm!` 内联汇编 (rusl 自建 syscall 层) |
| 命名空间管理 | `#define ctx __malloc_context` | Rust 模块天然隔离, 无需宏重映射 |
| 安全标记 | 无 (纯 C) | `unsafe` — 标记指针解引用操作 |