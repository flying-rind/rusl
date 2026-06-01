# free Rust 接口

## 复杂度分级: Level 3

> C 源文件: `src/malloc/mallocng/free.c`
> 对应 C spec: `src/malloc/mallocng/spec/free.md`
>
> 实现架构说明: `src/malloc/free.c` 仅为薄封装层，将 POSIX `free(void *p)` 转发给 `__libc_free`。
> 本模块是 `__libc_free` 的实际实现，通过 `glue.h` 中的 `#define free __libc_free` 重命名导出。
> 在 rusl 中，公共 `free()` 薄封装和对 `__libc_free` 的实际实现在同一模块中合并实现。

---

## [RELY]

```
__libc_free (对外导出, extern "C")
  │
  ├── crate::malloc::meta (内部模块, 重新设计)
  │     ├── struct Meta { ... }
  │     │     // 元数据结构体 (原 struct meta)
  │     │     // [Visibility]: Internal
  │     │     // 字段: prev, next (链表指针), mem (关联 Group),
  │     │     //       avail_mask, freed_mask (AtomicU32 位掩码),
  │     │     //       last_idx: u8 (5位), freeable: bool,
  │     │     //       sizeclass: u8 (6位), maplen: usize
  │     │
  │     ├── struct Group { ... }
  │     │     // 内存组结构体 (原 struct group)
  │     │     // [Visibility]: Internal
  │     │     // 字段: meta (指向 Meta), active_idx: u8 (5位), storage[]
  │     │
  │     ├── struct MetaArea { ... }
  │     │     // 元数据区结构体 (原 struct meta_area)
  │     │     // [Visibility]: Internal
  │     │     // 字段: check (u64, 应等于 ctx.secret),
  │     │     //       next (指向 MetaArea), nslots (i32), slots[]
  │     │
  │     ├── struct MallocContext { ... }
  │     │     // 全局分配器上下文 (原 struct malloc_context)
  │     │     // [Visibility]: Internal
  │     │     // 字段: secret, pagesize, init_done, mmap_counter,
  │     │     //       free_meta_head, avail_meta, avail_meta_count,
  │     │     //       meta_area_head/tail, active[48], usage_by_class[48],
  │     │     //       unmap_seq[32], bounces[32], seq, brk
  │     │
  │     ├── static ctx: MallocContext;
  │     │     // 全局分配器上下文实例 [Visibility]: Internal
  │     │     // 原 C 符号: __malloc_context (extern hidden, 定义于 malloc.c)
  │     │     // rust spec 中作为 meta 模块内部可变的全局静态
  │     │
  │     ├── static SIZE_CLASSES: [u16; 48];
  │     │     // 48 个大小类别的槽位容量表 (以 UNIT 为单位)
  │     │     // [Visibility]: Internal
  │     │     // 原 C 符号: __malloc_size_classes (extern hidden, 定义于 malloc.c)
  │     │
  │     ├── const UNIT: usize = 16;
  │     │     // 基本对齐单位 [Visibility]: Internal
  │     │
  │     ├── const IB: usize = 4;
  │     │     // In-band 头部大小 [Visibility]: Internal
  │     │
  │     ├── const MMAP_THRESHOLD: usize = 131052;
  │     │     // mmap 直接分配的阈值 [Visibility]: Internal
  │     │
  │     ├── unsafe fn get_meta(p: *const u8) -> &Meta;
  │     │     // 从分配指针反查元数据, 含多重安全断言
  │     │     // [Visibility]: Internal
  │     │     // 前置: p 为 16 字节对齐的有效分配指针
  │     │     // 后置: 返回 p 所属组的 Meta 引用; 校验失败则 panic
  │     │     // 原 C 函数: static inline get_meta() in meta.h
  │     │     // 校验链 (与原 C 一致):
  │     │     //   1. assert!(p.align_offset(16) == 0)  — 地址 16 字节对齐
  │     │     //   2. 从 p[-2] 读 16 位 offset, p[-3] & 31 得 slot index
  │     │     //   3. 若 p[-4] != 0, offset 实际存于 p[-8] (32-bit)
  │     │     //   4. base = p - UNIT*offset - UNIT, meta = (*base).meta
  │     │     //   5. assert!(meta.mem == base)  — 双向绑定
  │     │     //   6. assert!(index <= meta.last_idx)  — 索引不越界
  │     │     //   7. assert!(!(meta.avail_mask & (1<<index)))  — 未在可用集
  │     │     //   8. assert!(!(meta.freed_mask & (1<<index)))  — 未在释放集
  │     │     //   9. area = page_align_down(meta), assert!(area.check == ctx.secret)
  │     │     //  10. sizeclass 一致性校验
  │     │     //  11. maplen 边界校验
  │     │
  │     ├── fn get_slot_index(p: *const u8) -> usize;
  │     │     // 从 p[-3] & 31 提取槽位索引 (0..31)
  │     │     // [Visibility]: Internal
  │     │     // 原 C 函数: static inline get_slot_index() in meta.h
  │     │
  │     ├── fn get_stride(g: &Meta) -> usize;
  │     │     // 计算组内每个槽位的跨步大小
  │     │     // [Visibility]: Internal
  │     │     // Case 1 (mmap 单槽位): g.maplen * PGSZ - UNIT
  │     │     // Case 2 (常规 slab 组): UNIT * SIZE_CLASSES[g.sizeclass]
  │     │     // 原 C 函数: static inline get_stride() in meta.h
  │     │
  │     ├── unsafe fn get_nominal_size(p: *const u8, end: *const u8) -> usize;
  │     │     // 从 reserved 字段恢复原始分配大小, 并验证溢出守卫字节
  │     │     // [Visibility]: Internal
  │     │     // 前置: p 指向分配块起始, end 指向槽位末尾
  │     │     // 后置: 返回 用户可用字节数 (end - reserved - p)
  │     │     // 原 C 函数: static inline get_nominal_size() in meta.h
  │     │     //       兼作内存损坏检测 (校验 reserved 字段及溢出字节)
  │     │
  │     ├── fn free_meta(m: &mut Meta);
  │     │     // 将 meta 清零并归还到 ctx.free_meta_head 空闲链表
  │     │     // [Visibility]: Internal
  │     │     // 前置: 调用者持有锁, m 不再被使用
  │     │     // 后置: m 所有字段清零, 加入 free_meta_head 链表
  │     │     // 原 C 函数: static inline free_meta() in meta.h
  │     │
  │     ├── fn queue(head: &mut Option<NonNull<Meta>>, m: &mut Meta);
  │     │     // 将 meta 节点插入双向循环链表尾部
  │     │     // [Visibility]: Internal
  │     │     // 前置: m 不在任何链表中 (m.prev == m.next == None)
  │     │     // 后置 (空链表): m.prev = m.next = m (自环), head = Some(m)
  │     │     // 后置 (非空链表): m 插入到 head 之前 (循环链表尾部)
  │     │     // 原 C 函数: static inline queue() in meta.h
  │     │
  │     ├── fn dequeue(head: &mut Option<NonNull<Meta>>, m: &mut Meta);
  │     │     // 从双向循环链表中移除 meta 节点
  │     │     // [Visibility]: Internal
  │     │     // 前置: m 在 head 指向的链表中
  │     │     // 后置 (单节点): head = None, m.prev = m.next = None
  │     │     // 后置 (多节点): m 从链表中移除, 若 head == m 则 head 更新
  │     │     // 原 C 函数: static inline dequeue() in meta.h
  │     │
  │     ├── fn activate_group(m: &Meta) -> u32;
  │     │     // 通过原子 CAS 将 freed_mask 中已释放槽位移至 avail_mask
  │     │     // [Visibility]: Internal
  │     │     // 前置: 调用者持有锁, m.avail_mask == 0
  │     │     // 后置: avail_mask 包含 freed_mask 中 active_idx 范围内的位
  │     │     //       freed_mask 中被认领的位通过原子 CAS 清除
  │     │     //       返回新的 avail_mask 值
  │     │     // 原 C 函数: static inline activate_group() in meta.h
  │     │     // 依赖: AtomicU32::compare_exchange (即原 C a_cas)
  │     │
  │     ├── fn step_seq();
  │     │     // 推进全局操作序列计数器 ctx.seq
  │     │     // [Visibility]: Internal
  │     │     // 前置: 调用者持有锁
  │     │     // 后置 (ctx.seq == 255): ctx.seq = 1, 清零所有 unmap_seq[]
  │     │     // 后置 (ctx.seq < 255): ctx.seq += 1
  │     │     // 原 C 函数: static inline step_seq() in meta.h
  │     │
  │     ├── fn record_seq(sc: usize);
  │     │     // 记录 size class sc 最近一次 unmap 的序列号
  │     │     // [Visibility]: Internal
  │     │     // 后置: 若 7 <= sc < 39, ctx.unmap_seq[sc-7] = ctx.seq
  │     │     // 原 C 函数: static inline record_seq() in meta.h
  │     │
  │     ├── fn is_bouncing(sc: usize) -> bool;
  │     │     // 查询 size class sc 是否处于 "弹跳" 抖动状态
  │     │     // [Visibility]: Internal
  │     │     // 后置: 返回 ctx.bounces[sc-7] >= 100 (若 sc-7 < 32)
  │     │     // 原 C 函数: static inline is_bouncing() in meta.h
  │     │
  │     └── fn size_to_class(n: usize) -> usize;
  │           // 将字节大小映射到 size class 索引 (0..47)
  │           // [Visibility]: Internal
  │           // 原 C 函数: static inline size_to_class() in meta.h
  │           // 依赖: SIZE_CLASSES[], a_clz_32()
  │
  ├── crate::malloc::glue (内部模块)
  │     ├── fn use_madv_free() -> bool;
  │     │     // 编译期/运行时控制 MADV_FREE 开关
  │     │     // [Visibility]: Internal
  │     │     // rusl 默认返回 false (等价 USE_MADV_FREE=0)
  │     │     // 原 C 宏: #define USE_MADV_FREE 0 in glue.h
  │     │
  │     ├── fn is_multi_threaded() -> bool;
  │     │     // 运行时检测是否需要加锁
  │     │     // [Visibility]: Internal
  │     │     // 等价于原 C 宏 MT (= libc.need_locks)
  │     │     // rusl 中通过检查线程数标志位实现
  │     │
  │     ├── fn wrlock();
  │     │     // 获取 malloc 全局写锁
  │     │     // [Visibility]: Internal
  │     │     // 若 is_multi_threaded() 则 LOCK(__malloc_lock)
  │     │     // 单线程模式下为空操作
  │     │     // 原 C 函数: static inline wrlock() in glue.h
  │     │
  │     ├── fn unlock();
  │     │     // 释放 malloc 全局锁
  │     │     // [Visibility]: Internal
  │     │     // 若持有锁则 UNLOCK(__malloc_lock)
  │     │     // 原 C 函数: static inline unlock() in glue.h
  │     │
  │     └── static __malloc_lock: [i32; 1];
  │           // 全局互斥锁变量 [Visibility]: Internal
  │           // 原 C: __attribute__((__visibility__("hidden"))) extern int __malloc_lock[1]
  │           //      定义于 malloc.c 通过 LOCK_OBJ_DEF 宏展开
  │           // rusl 中重新设计为内部 Mutex 或 AtomicI32 自旋锁
  │
  ├── crate::atomic_support (内部模块, 重新设计)
  │     ├── fn atomic_fetch_or(p: &AtomicU32, v: u32) -> u32;
  │     │     // 原子 fetch_or 操作
  │     │     // [Visibility]: Internal
  │     │     // 等价于原 C a_or() in atomic.h
  │     │     // rusl 实现: AtomicU32::fetch_or(Ordering::AcqRel)
  │     │
  │     ├── fn atomic_cas(p: &AtomicU32, t: u32, s: u32) -> Result<u32, u32>;
  │     │     // 原子 compare-and-swap
  │     │     // [Visibility]: Internal
  │     │     // 等价于原 C a_cas() in atomic.h
  │     │     // rusl 实现: AtomicU32::compare_exchange_weak/strong
  │     │
  │     └── fn atomic_crash() -> !;
  │           // 断言失败时终止进程
  │           // [Visibility]: Internal
  │           // 等价于原 C a_crash() in atomic.h (__builtin_trap 或非法指令)
  │           // rusl 实现: core::intrinsics::abort() 或 unreachable!()
  │
  ├── crate::syscall (内部模块)
  │     ├── unsafe fn sys_madvise(addr: *mut c_void, len: usize, advice: i32) -> i32;
  │     │     // 发起 SYS_madvise 系统调用
  │     │     // [Visibility]: Internal
  │     │     // 原 C: madvise 经 glue.h 重定义为 __madvise
  │     │     // rusl no_std 实现: 通过 asm!("syscall") 直接发起
  │     │     // advice 参数: MADV_FREE = 8 (Linux x86-64)
  │     │
  │     └── unsafe fn sys_munmap(addr: *mut c_void, len: usize) -> i32;
  │           // 发起 SYS_munmap 系统调用
  │           // [Visibility]: Internal
  │           // 原 C: munmap 经 glue.h 重定义为 __munmap
  │           // rusl no_std 实现: 通过 asm!("syscall") 直接发起
  │
  ├── 外部常量 / 类型
  │     ├── core::ffi::c_void              // 等价于 C void
  │     ├── core::ptr::NonNull<T>          // Rust 非空指针抽象
  │     ├── core::sync::atomic::AtomicU32  // Rust 原子操作原语
  │     ├── core::sync::atomic::Ordering   // 内存排序语义
  │     ├── PGSZ: usize                    // 页大小 (编译期或运行时确定)
  │     └── MADV_FREE: i32 = 8            // Linux madvise 惰性释放标志
  │
  └── 递归依赖终止
        ├── get_meta / get_slot_index / get_stride / get_nominal_size
        │     — meta 模块内部函数, 其规约见 meta.h 的 Rust spec
        ├── free_meta / queue / dequeue / activate_group
        │     — meta 模块内部函数, 其规约见 meta.h 的 Rust spec
        ├── step_seq / record_seq / is_bouncing
        │     — meta 模块内部函数, 其规约见 meta.h 的 Rust spec
        ├── Meta / Group / MetaArea / MallocContext / UNIT / IB / SIZE_CLASSES
        │     — meta 模块内部类型和常量
        ├── ctx (MallocContext 全局实例) — 定义于 malloc 模块或 meta 模块
        ├── wrlock / unlock / is_multi_threaded / use_madv_free
        │     — glue 模块内部函数
        ├── __malloc_lock — glue 模块内部锁变量
        ├── atomic_fetch_or / atomic_cas / atomic_crash
        │     — atomic_support 模块内部函数
        ├── sys_madvise / sys_munmap — syscall 模块内部函数
        └── AtomicU32 / Ordering / NonNull — Rust core 库类型
```

---

## [GUARANTEE]

### 对外导出接口

```rust
// [Visibility]: Internal — musl 内部符号。
//   `__libc_free` 是为 `__libc_` 前缀的 libc 内部函数。
//   C 标准和 POSIX 均未定义此符号。用户程序通过 `stdlib.h` 使用无前缀的 `free()`。
//   公共 `free()` 定义于 `src/malloc/free.rs`, 仅为对 `__libc_free` 的薄封装。
// [ABI Compatibility]: extern "C", 参数布局与原 C 接口完全兼容
#[no_mangle]
pub unsafe extern "C" fn __libc_free(p: *mut core::ffi::c_void);
```

#### 前置条件

1. `p == core::ptr::null_mut()`, 或 `p` 是由同一分配器实例先前分配的、尚未释放的有效指针。
2. 分配器上下文 `ctx` 已正确初始化 (`ctx.init_done != 0`).
3. `p` 满足 16 字节对齐 (`(p as usize) % 16 == 0`, 由 `get_meta` 内部断言保证).
4. 在多线程环境中: 调用者无需持有任何锁 (本函数内部自行处理同步).

#### 后置条件

**Case 1 (`p == core::ptr::null_mut()`)**: 函数立即返回, 无任何操作。符合 C 标准要求的 NULL 无操作行为。

**Case 2 (`p != core::ptr::null_mut()`)**: 指针指向的内存被标记为可供后续分配重用。释放后 `p` 自身的值不变, 但变为悬垂指针, 再次解引用或释放均为未定义行为。

- 若满足条件, 对应物理页通过 `sys_munmap` 归还操作系统。
- `__malloc_lock` 在函数返回时处于解锁状态。
- `errno` 在函数返回时恢复为调用前的值 (`sys_madvise` 和 `sys_munmap` 可能修改 `errno`, 已被保存/恢复)。

#### 不变量

1. **errno 保持不变量**: `__libc_free` 的执行 (包括内部 `sys_madvise` 和 `sys_munmap`) 必须保证调用者的 `errno` 值不被改变。任何可能修改 `errno` 的 syscall 前后必须保存/恢复 `errno`。

2. **Double-free 防护不变量**: 阶段 2 将 `p[-3]` 设为 255, `p[-2]` 清零, 使得 `get_meta` 在二次释放时校验失败。同时阶段 4 的 `assert!(!(mask & self))` 捕获 group 内部的重复释放。

3. **锁最小化不变量**: 快速路径 (阶段 4) 通过原子 CAS 在无锁条件下完成非首个/非最后 slot 的释放, 仅在以下情况获取锁:
   - 首个释放 slot (需要将 group 加入活跃链表)
   - 最后一个释放 slot (可能需要释放整个 group)
   - 单 slot group 的释放

4. **弹跳抑制不变量**: 通过 `ctx.bounces[sc-7]`、`ctx.unmap_seq[sc-7]`、`ctx.seq` 追踪 size class 的 unmap 频率, 防止在分配/释放密集交替的模式下反复 mmap/munmap。

#### 系统算法 (Level 3)

##### 阶段 0: NULL 快速路径

```rust
if p.is_null() { return; }
```

##### 阶段 1: 元数据获取与校验

```rust
let g: &Meta = unsafe { get_meta(p) };
let idx: usize = get_slot_index(p);
let stride: usize = get_stride(g);
let start: *mut u8 = g.mem.storage_ptr().add(stride * idx);
let end: *mut u8 = start.add(stride - IB);
unsafe { get_nominal_size(p, end) };
```

- `get_meta(p)`: 通过 `p[-2]` (offset) 反查 `struct group *base`, 再由 `base->meta` 获取 meta。执行全面校验 (offset 范围、meta 校验和、mask 一致性等)。
- `get_slot_index(p)`: 提取 `p[-3] & 31` 作为 slot 索引。
- `get_stride(g)`: 计算 slot 跨度。
- `get_nominal_size(p, end)`: 解析存储大小, 校验 reserved 字段及溢出字节, 兼作内存损坏检测。
- 前置要求 `(p as usize) % 16 == 0`、`meta.mem == base`、`idx <= meta.last_idx`、slot 不在 freed/avail mask 中 (防止 double-free)。

##### 阶段 2: 头部失效化 (双重释放检测)

```rust
unsafe {
    *p.sub(3) = 255;                         // 标记为无效 (index=31, reserved=7)
    *(p.sub(2) as *mut u16) = 0;             // 清零 group 头部偏移量
}
```

- `p[-3] = 255`: slot 索引字段置为无效值, 使后续 `get_slot_index` 返回异常值。
- `*(p-2 as *mut u16) = 0`: 清零 group 头部偏移量, 使 `get_meta` 无法正确定位 group。
- 这两步确保任何对已释放指针的再释放 (double-free) 将在阶段 1 的断言校验中被捕获。

##### 阶段 3: 页粒度 MADV_FREE (已编译期禁用)

```rust
if (start.sub(1) as usize ^ end as usize) >= 2 * PGSZ && g.last_idx > 0 {
    let base: *mut u8 = start.add((-((start as isize) as usize)) & (PGSZ - 1));
    let len: usize = (end as usize - base as usize) & !(PGSZ - 1);
    if len > 0 && use_madv_free() {
        let e = errno_get();
        unsafe { sys_madvise(base as *mut c_void, len, MADV_FREE) };
        errno_set(e);
    }
}
```

- 仅在 slot 跨度至少 2 个页且非单 slot group 时触发。
- 计算 slot 内完整页的起始 (`base` 对齐到页边界) 和长度 (`len` 页对齐)。
- `MADV_FREE`: 告知内核可惰性回收这些页, 但在再次访问前数据仍有效。
- **`use_madv_free()` 当前返回 `false`**, 此路径无实际效果 (等价 `USE_MADV_FREE=0`)。

##### 阶段 4: 快速路径 (无锁原子释放)

```rust
let self_mask: u32 = 1u32 << idx;
let all_mask: u32 = (2u32 << g.last_idx) - 1;

// 原子 CAS 无锁循环
loop {
    let freed: u32 = g.freed_mask.load(Ordering::Acquire);
    let avail: u32 = g.avail_mask.load(Ordering::Acquire);
    let mask: u32 = freed | avail;

    assert!(!(mask & self_mask) != 0, "double-free detected");  // 防 double-free

    // 首个释放 或 最后一个被使用槽位 → 进入慢速路径
    if freed == 0 || mask.wrapping_add(self_mask) == all_mask {
        break;
    }

    // 无锁更新 freed_mask
    if !is_multi_threaded() {
        g.freed_mask.store(freed | self_mask, Ordering::Release);
        return;
    }
    let result = g.freed_mask.compare_exchange(
        freed, freed | self_mask,
        Ordering::AcqRel, Ordering::Acquire
    );
    if result.is_ok() {
        return;                    // CAS 成功, 释放完成
    }
    // CAS 失败, 重试
}
```

- **进入条件**: 组内已有其他已释放 slot (`freed != 0`) 且本 slot 不是最后一个 (`mask + self != all`)。
- **单线程** (`!is_multi_threaded()`): 直接原子写入 `freed_mask`。
- **多线程** (`is_multi_threaded()`): 使用 `AtomicU32::compare_exchange` (等价原 C 的 `a_cas`) 无锁更新 `freed_mask`。若 CAS 失败 (并发修改) 则重试。
- 快速路径**避免获取全局锁**, 大幅降低多线程释放竞争。

##### 阶段 5: 慢速路径 (持锁处理)

```rust
wrlock();
let mi: Option<MapInfo> = nontrivial_free(g, idx);
unlock();

if let Some(mapinfo) = mi {
    let e = errno_get();
    unsafe { sys_munmap(mapinfo.base as *mut c_void, mapinfo.len) };
    errno_set(e);
}
```

- `wrlock()`: 获取 `__malloc_lock` 写锁。
- `nontrivial_free(g, idx)`: 处理释放逻辑 (可能释放整个 group)。
- `unlock()`: 释放锁。
- 若返回 `Some(mapinfo)`: 调用 `sys_munmap` 归还物理内存。
- `errno` 在 `sys_munmap` 前后保存/恢复, 保证释放操作不污染调用者的 `errno`。

**异常安全**:
- 函数保证在返回时 `__malloc_lock` 处于解锁状态 (`wrlock`/`unlock` 配对)。
- 任何内部 `assert!` 失败将触发 `atomic_crash()` (或 `panic!`), 防止损坏状态扩散。

**复杂度**: 快速路径 O(1) 无锁; 慢速路径 O(1) + 可能的 group 释放递归。

---

### 内部依赖接口 (重新设计)

以下符号为 `__libc_free` 的内部实现依赖, 不对外导出, 可按 Rust 设计哲学完全重新设计。

---

#### MapInfo

```rust
// [Visibility]: Internal — 仅在 free 模块内部使用
// 替代原 C 的 struct mapinfo { void *base; size_t len; }
// 原 C 使用 {NULL, 0} 哨兵表示 "无需 unmap"
// Rust 重新设计为 Option<MapInfo>, 利用类型系统消解哨兵值
struct MapInfo {
    base: core::ptr::NonNull<u8>,
    len: usize,
}
```

**意图**: 用于在 `nontrivial_free` 和调用者之间传递需要 `munmap` 的内存范围信息。

- `None`: 无需 `munmap`。
- `Some(MapInfo{ base, len })`: 需要调用 `sys_munmap(base.as_ptr(), len)` 归还物理页。

---

#### nontrivial_free (内部函数, 重新设计)

```rust
// [Visibility]: Internal — pub(crate) within free module
// 原 C: static struct mapinfo nontrivial_free(struct meta *g, int i);
//
// Rust 重新设计:
//   - 输入: g 为 &Meta 引用 (不可变借用, 保证不悬挂)
//   - 输入: i 为 usize 而非 c_int
//   - 输出: Option<MapInfo> 替代哨兵值 mapinfo {0, 0}
//   - 掩码操作使用 u32 位运算, 比 C 的 signed int 移位更安全
fn nontrivial_free(g: &Meta, i: usize) -> Option<MapInfo>;
```

**描述**: 处理需要持有锁的 "非平凡" 释放操作。标记 slot `i` 为已释放, 并在适当条件下对整个 group 执行释放或将其加入活跃链表。由 `__libc_free` 慢速路径及 `free_group` 递归调用。

**前置条件**:
- `g` 指向有效 `Meta`, 且 slot `i` 当前未在 `freed_mask` 或 `avail_mask` 中。
- 调用者持有 `__malloc_lock` 写锁。
- `i` 在 `[0, g.last_idx]` 范围内。
- `g.sizeclass < 48` (多 slot group 的 sizeclass 范围; 单 slot group 也可能经此路径但需满足特定条件)。

**处理流程**:

1. **全组空闲检测**:
   若 `mask + self == all` (即本 slot 释放后组内无任何活跃分配) 且 `okay_to_free(g)` 为真:
   - **出队处理** (若 group 在活跃链表中):
     - `activate_new = (ctx.active[sc] == g)`。
     - `dequeue(&ctx.active[sc], g)`: 从活跃链表移除。
     - 若移除的是当前活跃 group 且链表非空, 调用 `activate_group(ctx.active[sc])` 激活下一个 group。
   - 返回 `free_group(g)` 的结果 (可能为 `Some(MapInfo)` 或 `None`)。

2. **首次释放检测**:
   若 `mask == 0` (此前组内无任何 freed/available slot):
   - 若该 group 尚未在活跃链表中, 则 `queue(&ctx.active[sc], g)` 将其加入链表首部。

3. **标记释放**: 无论上述条件是否满足, 最终执行 `atomic_fetch_or(&g.freed_mask, self)` 原子设置 freed 标记。

**后置条件**:
- `g.freed_mask` 的第 `i` 位必定被设置。
- 若触发全组释放: group `g` 已通过 `free_group` 回收, 可能触发 `munmap`。
- 若触发首次释放: group `g` 位于 `ctx.active[sc]` 链表中。
- 返回: `None` (无需 unmap) 或 `Some(MapInfo)` (需要 `munmap`)。

**复杂度**: O(1), 不含 `free_group` 递归。

---

#### free_group (内部函数, 重新设计)

```rust
// [Visibility]: Internal — pub(crate) within free module
// 原 C: static struct mapinfo free_group(struct meta *g);
//
// Rust 重新设计:
//   - 输入: g 为 &Meta 引用
//   - 输出: Option<MapInfo> 替代哨兵值
//   - g.maplen 直接用 usize 判断, 无 C 的隐式转换
//   - 递归调用 nontrivial_free 的返回值直接传播
fn free_group(g: &Meta) -> Option<MapInfo>;
```

**描述**: 释放一个 group 的全部资源。根据 group 的类型采取不同策略:
- **独立 mmap 组** (`g.maplen > 0`): 记录内存区域用于后续 `munmap`。
- **嵌套组** (`g.maplen == 0`, 嵌入在另一个 group 的 slot 中): 递归释放该 slot 所属的父 group。

**前置条件**:
- `g` 是一个有效的、可以释放的 group (已通过 `okay_to_free` 判定或通过 `nontrivial_free` 条件触发)。
- 调用者持有 `__malloc_lock` 写锁。
- `g.mem.meta` 的指针等价于 `g` (group 与 meta 双向关联有效)。

**处理流程**:

1. **更新使用统计**: 若 `sc < 48`, `ctx.usage_by_class[sc] -= g.last_idx + 1`.
2. **独立 mmap 组路径** (`g.maplen > 0`):
   - `step_seq()`: 递增全局序列号。
   - `record_seq(sc)`: 记录该 size class 最近一次 unmap 的序列号, 用于弹跳检测。
   - 返回 `Some(MapInfo { base: g.mem.as_ptr(), len: g.maplen * PGSZ })`.
3. **嵌套组路径** (`g.maplen == 0`):
   - `p = g.mem`: 获取嵌套组基址。
   - `m = get_meta(p)`: 反查父 group 的 meta。
   - `idx = get_slot_index(p)`: 获取该 slot 在父 group 中的索引。
   - `g.mem.meta = core::ptr::null_mut()`: 断开 group→meta 关联, 防止悬挂指针。
   - 递归调用 `nontrivial_free(m, idx)` 释放父 group 中对应 slot。
   - 返回递归结果 (直接传播)。
4. **回收 meta**: `free_meta(g)` 将 `g` 归还到 `ctx.free_meta_head` 空闲链表。

**后置条件**:
- `g` 已被回收 (`free_meta`), 不可再访问。
- 若 `g.maplen > 0`: 返回的 `Some(MapInfo)` 包含需要 `munmap` 的内存范围。
- 若 `g.maplen == 0`: 父 group 对应 slot 已标记为 freed, 返回值取决于递归路径是否需要 `munmap`。

**复杂度**: O(1) + 可能的递归 `nontrivial_free`。

---

#### okay_to_free (内部函数, 重新设计)

```rust
// [Visibility]: Internal — fn within free module
// 原 C: static int okay_to_free(struct meta *g);
//
// Rust 重新设计:
//   - 返回 bool 而非 int (0/1)
//   - 利用 Rust 的类型安全避免 C 中隐式类型转换风险
//   - 判定逻辑用 match/if-let 表达, 比 C 的级联 if-return 更清晰
fn okay_to_free(g: &Meta) -> bool;
```

**描述**: 判断一个已完全释放 (所有 slot 均 freed/available) 的 group `g` 是否应当归还操作系统 (通过 `free_group` → `sys_munmap`)。仅由 `nontrivial_free` 在检测到 group 完全空闲时调用。

**前置条件**:
- `g` 指向一个有效的 `Meta`, 其所属的所有 slot 的 `freed_mask | avail_mask == (2u32 << g.last_idx) - 1` (即全部 slot 已释放或可用)。
- 调用者持有 `__malloc_lock` 写锁。
- `g.sizeclass` 有效 (< 64)。

**判定逻辑 (优先级递减, 与 C 的 7 层决策级联等价)**:

1. **不可释放组**: 若 `!g.freeable` → 返回 `false` (保留组, 后续分配复用)。
2. **大尺寸单 slot mmap** (`sc >= 48`): 总是返回 `true`, 因为大规模 mmap 不适合 slot 复用。
3. **非标准 stride 的组**: 若 `get_stride(g) < UNIT * SIZE_CLASSES[sc]` → 返回 `true`, 此类组无法正常放入 slot 分配体系。
4. **嵌套组** (`maplen == 0`): 组内存在另一个 group 的 slot 内 → 返回 `true`。重建开销低, 且可能阻塞更大队列的释放。
5. **活跃链表中存在其他组** (`g.next != g`): → 返回 `true`。释放当前组以合并未来分配, 减少碎片。
6. **非弹跳 size class**: `!is_bouncing(sc)` → 返回 `true`。非弹跳 class 的 group 可以安全释放。
7. **低容量组在高使用率弹跳 class**:
   - 计算 `cnt = g.last_idx + 1` (组内 slot 数)
   - 计算 `usage = ctx.usage_by_class[sc]` (该 class 累计分配数)
   - 若 `9 * cnt <= usage && cnt < 20` → 返回 `true`。使用率足够高, 说明需要更大容量的组, 释放此低容量组以便后续分配新的大容量组。
8. **保底策略**: 返回 `false` — 在弹跳 class 中保留最后一个 group 供快速复用, 避免频繁 mmap/munmap 抖动。

**后置条件**:
- 返回 `false`: 调用者不会释放该 group; `freed_mask` 将被设置, group 保留供后续 `malloc` 复用。
- 返回 `true`: 调用者将继续执行 `free_group(g)`, 最终可能 `munmap` 归还内存。

**复杂度**: O(1), 纯判断逻辑。

---

## 关键不变量

### I1: errno 保持不变量

`__libc_free` 的执行 (包括内部 `sys_madvise` 和 `sys_munmap` 系统调用) 必须保证调用者的 `errno` 值不被改变。任何可能修改 `errno` 的 syscall 前后必须保存/恢复 `errno`。

### I2: 元数据完整性不变量

在任何线程释放操作前后, 以下性质始终成立:
- 若 `p` 是有效的已分配指针, 则 `get_meta(p)` 能成功定位到正确的 `Meta`, 且 `MetaArea.check == ctx.secret`
- 一个槽位不能同时出现在 `freed_mask` 和 `avail_mask` 中
- `ctx.active[sc]` 链表上的每个 group, 其 `avail_mask` 必须非零 (即组内有可用槽位)

### I3: 锁层级不变量

- `nontrivial_free`、`free_group`、`okay_to_free` 必须在持有 `__malloc_lock` 写锁时调用
- 快速路径原子 CAS 路径不持有锁, 通过 `AtomicU32::compare_exchange` 保证 `freed_mask` 更新的原子性

### I4: usage_by_class 一致性不变量

`ctx.usage_by_class[sc]` 应等于所有 sc 类的活动组中 `last_idx + 1` 的和。当组被 `free_group` 释放时, 其贡献从计数中扣减。

### I5: 单槽 mmap 组不变量

当 `g.last_idx == 0 && g.maplen > 0` (单槽 mmap 组) 时, 释放该槽位必定触发整组 `munmap`, 因此不会走 `madvise(MADV_FREE)` 页面回收路径。

---

## 符号导出状态

| 符号 | Rust 表示 | 导出状态 | 说明 |
|------|----------|---------|------|
| `__libc_free` | `extern "C" fn(*mut c_void)` | **Internal (musl 内部符号)** | 实际实现符号; 公共 `free()` 薄封装转发至此 |
| `MapInfo` | `struct MapInfo` | **Internal (不导出)** | 仅在 free 模块内部定义和使用 |
| `nontrivial_free` | `fn(&Meta, usize) -> Option<MapInfo>` | **Internal (模块私有)** | 仅在 free 模块内可见 |
| `free_group` | `fn(&Meta) -> Option<MapInfo>` | **Internal (模块私有)** | 仅在 free 模块内可见 |
| `okay_to_free` | `fn(&Meta) -> bool` | **Internal (模块私有)** | 仅在 free 模块内可见 |

---

## 跨文件依赖汇总

| 依赖符号 | Rust 类型/表示 | 来源模块 | 可见性 |
|---------|---------------|---------|--------|
| `__libc_free` | `extern "C" fn(*mut c_void)` | free 模块 | **Internal** (musl 内部) |
| `Meta` | `struct Meta` | meta 模块 | Internal |
| `Group` | `struct Group` | meta 模块 | Internal |
| `MetaArea` | `struct MetaArea` | meta 模块 | Internal |
| `MallocContext` | `struct MallocContext` | meta 模块 | Internal |
| `ctx` | `static MallocContext` | meta/malloc 模块 | Internal |
| `SIZE_CLASSES` | `static [u16; 48]` | meta/malloc 模块 | Internal |
| `UNIT` | `const usize = 16` | meta 模块 | Internal |
| `IB` | `const usize = 4` | meta 模块 | Internal |
| `MMAP_THRESHOLD` | `const usize = 131052` | meta 模块 | Internal |
| `get_meta` | `unsafe fn(*const u8) -> &Meta` | meta 模块 | Internal |
| `get_slot_index` | `fn(*const u8) -> usize` | meta 模块 | Internal |
| `get_stride` | `fn(&Meta) -> usize` | meta 模块 | Internal |
| `get_nominal_size` | `unsafe fn(*const u8, *const u8) -> usize` | meta 模块 | Internal |
| `free_meta` | `fn(&mut Meta)` | meta 模块 | Internal |
| `queue` | `fn(&mut Option<NonNull<Meta>>, &mut Meta)` | meta 模块 | Internal |
| `dequeue` | `fn(&mut Option<NonNull<Meta>>, &mut Meta)` | meta 模块 | Internal |
| `activate_group` | `fn(&Meta) -> u32` | meta 模块 | Internal |
| `step_seq` | `fn()` | meta 模块 | Internal |
| `record_seq` | `fn(usize)` | meta 模块 | Internal |
| `is_bouncing` | `fn(usize) -> bool` | meta 模块 | Internal |
| `size_to_class` | `fn(usize) -> usize` | meta 模块 | Internal |
| `use_madv_free` | `fn() -> bool` | glue 模块 | Internal |
| `is_multi_threaded` | `fn() -> bool` | glue 模块 | Internal |
| `wrlock` | `fn()` | glue 模块 | Internal |
| `unlock` | `fn()` | glue 模块 | Internal |
| `__malloc_lock` | `static` (Mutex 或 AtomicI32) | glue 模块 | Internal |
| `atomic_fetch_or` | `fn(&AtomicU32, u32) -> u32` | atomic_support 模块 | Internal |
| `atomic_cas` | `fn(&AtomicU32, u32, u32) -> Result<u32, u32>` | atomic_support 模块 | Internal |
| `atomic_crash` | `fn() -> !` | atomic_support 模块 | Internal |
| `sys_madvise` | `unsafe fn(*mut c_void, usize, i32) -> i32` | syscall 模块 | Internal |
| `sys_munmap` | `unsafe fn(*mut c_void, usize) -> i32` | syscall 模块 | Internal |
| `PGSZ` | `usize` (常量或运行时值) | 系统配置 | Internal |
| `MADV_FREE` | `const i32 = 8` | Linux 系统常量 | Internal |
| `errno_get` / `errno_set` | `fn() -> i32` / `fn(i32)` | 错误处理模块 | Internal |
| `AtomicU32` | `core::sync::atomic::AtomicU32` | Rust core 库 | Public |
| `Ordering` | `core::sync::atomic::Ordering` | Rust core 库 | Public |
| `NonNull<T>` | `core::ptr::NonNull<T>` | Rust core 库 | Public |
| `c_void` | `core::ffi::c_void` | Rust core 库 | Public |

---

## rusl no_std 适配说明

1. **无 `libc` crate**: 所有 C ABI 类型使用 `core::ffi::c_void`、`usize` (等价 `size_t`)、`u8`/`u32` 等 Rust 原生类型。`c_int` 用 `i32` 替代。

2. **no_std 约束**: 不依赖 `std::alloc`; 释放操作仅修改内部数据结构, 无需标准库支持。

3. **原子操作**: 原 C 的 `a_cas`/`a_or` 通过 `core::sync::atomic::AtomicU32` 的 `compare_exchange`/`fetch_or` 方法实现, 零外部依赖。

4. **断言/崩溃**: 原 C 的 `assert` → `a_crash` 链路在 Rust 中可重新设计:
   - debug 模式使用 `assert!` 宏 (等价 C 的 NDEBUG 控制)
   - release 模式使用 `core::hint::unreachable_unchecked()` 或自定义 `abort()` 实现
   - rusl 要求即使 release 模式也做安全校验 (与 musl `USE_REAL_ASSERT` 未定义时的默认行为一致)

5. **内联汇编替代 syscall 封装**: 内部的 `sys_madvise` 和 `sys_munmap` 通过 `asm!("syscall")` 直接发起, 不经过任何外部 libc FFI 封装。

6. **`errno` 机制**: rusl 需自行实现 thread-local `errno` 存储, 不依赖外部 `libc` 的 `__errno_location`。`errno_get()`/`errno_set()` 为内部 thin wrapper。

7. **`MapInfo` 哨兵值消除**: 原 C 使用 `{NULL, 0}` 哨兵表示 "无需 unmap"。Rust 重新设计为 `Option<MapInfo>`, 利用类型系统在编译期消除哨兵值错误使用的风险。

8. **锁实现**: `wrlock`/`unlock` 基于内部的 `__malloc_lock` (Rust 重新设计为自旋锁或 futex-based 锁)。单线程模式下 (`!is_multi_threaded()`) 所有锁操作为空操作, 零开销。

---

## 内部函数 Rust 重新设计要点

| C 原始设计 | Rust 重新设计 | 设计理由 |
|-----------|-------------|---------|
| `struct mapinfo { void *base; size_t len; }` 哨兵值 `{0,0}` | `Option<MapInfo> { base: NonNull<u8>, len: usize }` | 利用类型系统消除哨兵值, 编译期防止空指针误用 |
| `static struct mapinfo nontrivial_free(...)` 返回哨兵 | `fn(...) -> Option<MapInfo>` | `Option` 语义明确: `None` = 无需 unmap |
| `static int okay_to_free(...)` 返回 0/1 | `fn(...) -> bool` | `bool` 比 `i32` 更精确表达二值语义 |
| `a_or(&g->freed_mask, self)` | `g.freed_mask.fetch_or(self, Ordering::AcqRel)` | 使用 Rust 标准原子类型, 附带显式内存排序 |
| `a_cas(&g->freed_mask, freed, freed+self)` | `g.freed_mask.compare_exchange(freed, freed\|self, ...)` | Rust 标准库原子 CAS, 类型安全 |
| `assert(!(mask & self))` → `a_crash()` | `debug_assert!` + 运行时 `abort()` | Rust 的 panic 机制可被 no_std 自定义处理 |
| `wrlock()` / `unlock()` 宏 | `fn wrlock()` / `fn unlock()` | Rust 函数比 C 宏更安全, 有类型检查和可见性控制 |
| `MT` 宏 (`libc.need_locks`) | `fn is_multi_threaded() -> bool` | 函数替代宏, 可测试性更好 |
| `USE_MADV_FREE` 编译期宏 | `fn use_madv_free() -> bool` | 运行时可配置, 未来可支持动态切换 |