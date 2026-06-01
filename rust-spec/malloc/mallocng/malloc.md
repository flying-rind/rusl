# malloc Rust 接口

> C 源文件: `src/malloc/mallocng/malloc.c`
> 对应 C spec: `src/malloc/mallocng/spec/malloc.md`
> musl 内部编译符号: `__libc_malloc_impl` (通过 `glue.h` 中 `#define malloc __libc_malloc_impl` 重命名)

---

## [RELY]

```
malloc (对外导出, extern "C", #[no_mangle])
├── crate::malloc::meta (内部模块, 定义于 meta.rs)
│     // --- 核心数据结构 (Rust 重新设计) ---
│     ├── struct Group { ... }
│     │     // 分配组头部 (原 struct group)
│     │     // 字段: meta: *const Meta, active_idx: u8 (低5位有效), storage[]
│     │     // 语义: 一组固定大小 slot 的容器, 头部占 UNIT 字节
│     │     // 不变量: group.meta.as_ref().mem == group
│     │
│     ├── struct Meta { ... }
│     │     // 分配组元数据 (原 struct meta)
│     │     // 字段: prev: *const Meta, next: *const Meta (双向循环链表)
│     │     //       mem: *const Group (关联的 group)
│     │     //       avail_mask: AtomicI32 (可用槽位位掩码)
│     │     //       freed_mask: AtomicI32 (已释放槽位位掩码)
│     │     //       last_idx: u8 (最大槽位索引, 0..31)
│     │     //       freeable: bool (组是否可整体释放)
│     │     //       sizeclass: u8 (大小类别, 0..47 或 63)
│     │     //       maplen: usize (mmap 分配时的页数, 0=非 mmap)
│     │     // 不变量: avail_mask & freed_mask == 0 (同一槽位不共存)
│     │     //
│     │     // Rust 设计要点:
│     │     // - avail_mask/freed_mask 使用 AtomicI32 替代 C 的 volatile int + a_cas
│     │     // - 位域打包用 Rust 的位操作 + 访问器方法实现
│     │
│     ├── struct MetaArea { ... }
│     │     // 元数据区 (原 struct meta_area)
│     │     // 字段: check: u64, next: *const MetaArea, nslots: usize, slots[]
│     │     // 不变量: area.check == ctx.secret, 页对齐 (4KB)
│     │
│     ├── struct MallocContext { ... }
│     │     // 全局分配器上下文 (原 struct malloc_context)
│     │     // 字段: secret: u64, pagesize: usize, init_done: AtomicBool
│     │     //       mmap_counter: AtomicU32, free_meta_head: *mut Meta
│     │     //       avail_meta: *mut Meta, avail_meta_count: usize
│     │     //       avail_meta_area_count: usize, meta_alloc_shift: usize
│     │     //       meta_area_head: *mut MetaArea, meta_area_tail: *mut MetaArea
│     │     //       avail_meta_areas: *mut u8
│     │     //       active: [*const Meta; 48] (48个尺寸类别的活跃链表)
│     │     //       usage_by_class: [usize; 48] (slot使用计数)
│     │     //       unmap_seq: [u8; 32], bounces: [u8; 32], seq: u8
│     │     //       brk: usize (brk值, usize::MAX表示brk失效)
│     │     //
│     │     // Rust 设计要点:
│     │     // - init_done 使用 AtomicBool 替代 C 的 int + 依赖写锁的隐式同步
│     │     // - mmap_counter 使用 AtomicU32 替代 C 的 unsigned, 实现 lock-free 快照读取
│     │     // - active 数组项用 *const Meta 裸指针 (分配器内部, 不涉及生命周期)
│     │     // - seq/bounces/unmap_seq 为普通字段, 访问需持有锁
│     │
│     // --- 内部常量 ---
│     ├── const UNIT: usize = 16;
│     │     // 最小分配单元, 所有分配的字节对齐粒度
│     │
│     ├── const IB: usize = 4;
│     │     // In-band header 大小, 每个 slot 末尾的元数据开销
│     │
│     ├── const MMAP_THRESHOLD: usize = 131052;
│     │     // 超过此值的分配绕过 slab 机制, 直接使用 mmap
│     │
│     └── const PGSZ: fn() -> usize;
│           // 页大小: 若编译期定义 PAGESIZE 则直接返回常量, 否则返回 ctx.pagesize
│
├── crate::malloc::meta (内部函数, inline → pub(crate))
│     // --- 链表操作 (纯函数, 安全 Rust) ---
│     ├── unsafe fn queue(head: *mut *const Meta, m: *mut Meta);
│     │     // 将 Meta 节点 m 插入 *head 循环链表的尾部
│     │     // 前置: m 不在任何链表中 (m.prev == null && m.next == null)
│     │     // 后置 (空链表): m 自环, *head = m
│     │     // 后置 (非空): m 插入到 *head 之前, 循环完整性保持
│     │     // Rust 安全考虑: 内部裸指针操作, 需 unsafe 标注
│     │
│     ├── unsafe fn dequeue(head: *mut *const Meta, m: *mut Meta);
│     │     // 从 *head 循环链表中移除 Meta 节点 m
│     │     // 后置 (最后一个节点): *head = null, m.prev = m.next = null
│     │     // 后置 (多节点): m 从链表移除, 前后节点正确重链
│     │
│     ├── unsafe fn dequeue_head(head: *mut *const Meta) -> *mut Meta;
│     │     // 取出并返回 *head 循环链表的头节点, 委托给 dequeue()
│     │     // 后置 (空链表): 返回 null
│     │
│     // --- 元数据生命周期管理 ---
│     ├── unsafe fn free_meta(m: *mut Meta);
│     │     // 将 meta 清零并回收到 ctx.free_meta_head 空闲链表
│     │     // 依赖: queue()
│     │
│     // --- 槽位激活 ---
│     ├── unsafe fn activate_group(m: *mut Meta) -> u32;
│     │     // 原子地将 freed_mask 中 active_idx 范围内的 slot 转移到 avail_mask
│     │     // 前置: m.avail_mask == 0, 持有锁
│     │     // 后置: avail_mask 包含原 freed_mask 的可激活位, freed_mask 对应位已清除
│     │     // 返回: 更新后的 avail_mask
│     │     // 依赖: core::sync::atomic::AtomicI32::compare_exchange (替代 C 的 a_cas)
│     │
│     // --- 指针 → 元数据逆向解析 (安全关键函数) ---
│     ├── unsafe fn get_slot_index(p: *const u8) -> usize;
│     │     // 从 p[-3] & 31 提取槽位索引 (0..31)
│     │     // 前置: p 为 16 字节对齐的有效分配指针
│     │
│     ├── unsafe fn get_meta(p: *const u8) -> &Meta;
│     │     // 从分配指针逆向推导对应的 Meta (核心安全校验函数)
│     │     // 校验链: 16B对齐 → 偏移量解析 → group基址定位 → meta反查
│     │     //        → meta.mem==base → index<=last_idx → slot不在avail/freed中
│     │     //        → meta_area.check==ctx.secret → 偏移量与sizeclass一致性
│     │     // 任一断言失败 → a_crash() 立即终止进程
│     │     // Rust 安全考虑: 返回引用是安全的, 因为 Meta 在 meta_area 页中
│     │     //   永不释放; 但仍然标注 unsafe 因为解析过程依赖指针合法性
│     │
│     ├── unsafe fn get_nominal_size(p: *const u8, end: *const u8) -> usize;
│     │     // 解码分配块的原始请求大小
│     │     // reserved = p[-3] >> 5 (高3位); 若 reserved>=5, 实际值存于 end[-4]
│     │     // 返回: end - reserved - p (用户实际可用字节数)
│     │     // 断言: end[-reserved]==0, *end==0 (哨兵/溢出检测字节)
│     │
│     // --- 槽位大小计算 ---
│     ├── fn get_stride(g: &Meta) -> usize;
│     │     // 计算 group 中每个 slot 的跨度
│     │     // Case1 (mmap单槽): g.maplen * PGSZ - UNIT
│     │     // Case2 (常规slab): UNIT * SIZE_CLASSES[g.sizeclass]
│     │
│     // --- 大小编码 ---
│     ├── unsafe fn set_size(p: *mut u8, end: *mut u8, n: usize);
│     │     // 在分配块 in-band header 中写入请求大小 n
│     │     // reserved = end - p - n; 若 reserved>0 设 end[-reserved]=0
│     │     // 若 reserved>=5: 扩展编码, end[-5]=0, end[-4]存32bit reserved值
│     │     // p[-3] = (p[-3] & 31) | ((reserved & 7) << 5)
│     │
│     // --- 槽位 enframing (分配块构造) ---
│     ├── unsafe fn enframe(g: &Meta, idx: usize, n: usize, ctr: usize) -> *mut u8;
│     │     // 在指定槽位构造新分配块 (malloc 的最终输出步骤)
│     │     // 1) stride = get_stride(g); slack = (stride-IB-n)/UNIT
│     │     // 2) 计算随机化偏移 off (基于 ctr&255 或 p[-3] 递增)
│     │     // 3) 若 off>slack, 压缩到 slack 范围内
│     │     // 4) 若 off>0: 在 (p-2) 存偏移, p[-3]=7<<5, 推进 p
│     │     // 5) *(p-2)=offset, p[-3]=idx, set_size(p,end,n)
│     │     // 返回: 用户可用指针 p
│     │     // 依赖: get_stride(), set_size()
│     │
│     // --- 大小分类 ---
│     ├── fn size_to_class(n: usize) -> usize;
│     │     // 将字节大小 n 映射到 sizeclass 索引 (0..47)
│     │     // 算法: n=(n+IB-1)>>4; 若 n<10 直接返回 n
│     │     //      否则 n++, 用 leading_zeros (替代 a_clz_32) 定位 + 查表修正
│     │     // 依赖: SIZE_CLASSES[], u32::leading_zeros() (替代 a_clz_32)
│     │
│     ├── fn size_overflows(n: usize) -> bool;
│     │     // 检查 n >= usize::MAX/2 - 4096 (溢出安全边界)
│     │     // 若溢出: 设置 errno=ENOMEM, 返回 true
│     │     // 若安全: 返回 false
│     │
│     // --- 反碎片化序列号系统 ---
│     ├── fn step_seq();
│     │     // 推进 ctx.seq; 若 ctx.seq==255, 回绕到1并清零所有 unmap_seq[]
│     │     // 前置: 持有 malloc 锁 (wrlock)
│     │
│     ├── fn record_seq(sc: usize);
│     │     // 若 sc-7 < 32, ctx.unmap_seq[sc-7] = ctx.seq
│     │     // 记录 sc 最近一次 unmap 的序列号
│     │
│     ├── fn account_bounce(sc: usize);
│     │     // 检测并记录 map/unmap 抖动:
│     │     // 若上次记录序列号非零且 ctx.seq - seq < 10 → ctx.bounces[sc-7]++ (上限150)
│     │
│     ├── fn decay_bounces(sc: usize);
│     │     // 每次成功分配时衰减: 若 ctx.bounces[sc-7] > 0 → ctx.bounces[sc-7]--
│     │
│     └── fn is_bouncing(sc: usize) -> bool;
│           // 若 ctx.bounces[sc-7] >= 100 → true (处于弹跳状态, 推迟释放)
│           // 否则返回 false
│
├── crate::malloc::glue (内部模块, 定义于 glue.rs)
│     // --- 锁原语 (Rust 重新设计) ---
│     ├── fn rdlock();
│     │     // 获取读锁 (实质为排他锁, 因 RDLOCK_IS_EXCLUSIVE=1)
│     │     // 单线程环境 (need_locks==false): 无操作
│     │     // 多线程环境: 获取 __malloc_lock 自旋锁
│     │     // Rust 实现: 使用 Mutex<()> 或基于 AtomicBool 的自旋锁
│     │     //   (rusl 不能依赖 std::sync::Mutex, 需自行实现或使用 spin crate)
│     │
│     ├── fn wrlock();
│     │     // 获取写锁, 与 rdlock() 实现相同
│     │
│     ├── fn unlock();
│     │     // 释放锁; 前置: 当前线程持有锁
│     │
│     └── fn upgradelock();
│           // 锁升级 (当前为空操作, 因 RDLOCK_IS_EXCLUSIVE=1)
│           // 保留接口以备将来真正的读写锁实现
│
├── crate::malloc::glue (系统调用封装)
│     // 以下 syscall 通过 asm! 内联汇编直接发起, 不经过 libc crate
│     ├── unsafe fn __brk(addr: usize) -> usize;
│     │     // 封装 SYS_brk, 返回新的 program break 地址
│     │     // Rust 实现: asm!("syscall", ...) with SYS_brk
│     │
│     ├── unsafe fn __mmap(addr: usize, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
│     │     // 封装 SYS_mmap, 返回映射地址或 MAP_FAILED
│     │     // Rust 实现: asm!("syscall", ...) with SYS_mmap
│     │
│     ├── unsafe fn __mprotect(addr: *const u8, len: usize, prot: i32) -> i32;
│     │     // 封装 SYS_mprotect, 返回 0 或 -errno
│     │
│     ├── unsafe fn __munmap(addr: *const u8, len: usize) -> i32;
│     │     // 封装 SYS_munmap, 返回 0 或 -errno
│     │
│     └── unsafe fn __madvise(addr: *const u8, len: usize, advice: i32) -> i32;
│           // 封装 SYS_madvise, 返回 0 或 -errno
│           // 注: USE_MADV_FREE=0 时此函数在 free 中不被调用
│
├── crate::malloc::glue (随机密钥生成)
│     └── fn get_random_secret() -> u64;
│           // 生成运行时随机密钥, 用于 meta_area.check 校验
│           // 两步混合: 1) 栈地址*1103515245; 2) AT_RANDOM 中 memcpy 8字节
│           // Rust 实现: 可通过 asm! 读取 auxv 或使用 rdrand 指令
│           // 前置: auxv 已初始化 (动态链接器设置)
│
├── crate::malloc::meta (全局数据, 定义于 malloc 模块)
│     ├── static SIZE_CLASSES: [u16; 48];
│     │     // 大小类别查找表 (原 size_classes[])
│     │     // 尺寸: class 0-7: 1-8, class 8-11: 9-15, class 12-47: 18-8191
│     │     // 语义: SIZE_CLASSES[sc] 表示该类别下每个 slot 的 UNIT 数
│     │     // 可见性: pub(crate), 被 free/realloc/aligned_alloc/malloc_usable_size 引用
│     │
│     └── static mut CTX: MallocContext;
│           // 全局分配器上下文 (原 ctx)
│           // 初始全零 (init_done==false 表示未初始化)
│           // 可见性: pub(crate), 被所有 mallocng 模块共享
│           // Rust 安全考虑: 标记为 static mut, 所有访问需在 unsafe 块中
│           //   或使用 UnsafeCell / Mutex 包装以提升安全性
│
├── crate::malloc::free (内部模块, 定义于 free.rs)
│     └── unsafe fn nontrivial_free(g: *mut Meta, i: usize) -> MapInfo;
│           // 处理需要持有锁的"非平凡"释放操作 (来自 free.c)
│           // 递归依赖: alloc_group → ... → free_group 在嵌套组场景下重新调用
│           // Rust spec: 见 free.md 的 Rust spec
│
├── crate::malloc (本模块内部, 私有 static 数据)
│     ├── static SMALL_CNT_TAB: [[u8; 3]; 9];
│     │     // 小尺寸类别 (sc<9) 的 slot 数量表
│     │     // 每个 sc 有 3 个使用等级 (i=0 最少, i=2 最多)
│     │     // 原 C: static const uint8_t small_cnt_tab[][3]
│     │     // Rust: static 编译期常量, 模块私有
│     │
│     └── static MED_CNT_TAB: [u8; 4];
│           // 中等尺寸类别 (sc>=9) 的基础 slot 数
│           // 按 sc&3 索引: sc%4=0→28, =1→24, =2→20, =3→32
│           // 原 C: static const uint8_t med_cnt_tab[4]
│           // Rust: static 编译期常量, 模块私有
│
├── 外部依赖 (Rust core / no_std 环境)
│     ├── core::ffi::c_void (等价于 C 的 void)
│     ├── core::ffi::c_int (等价于 C 的 int, 用于 errno)
│     ├── core::sync::atomic::{AtomicI32, AtomicU32, AtomicBool, Ordering}
│     │     // 替代 C 的 volatile int + a_cas/a_or 原子操作
│     │     // a_cas(p,t,s) → p.compare_exchange(t, s, AcqRel, Acquire)
│     │     // a_or(p,v)    → p.fetch_or(v, AcqRel)
│     │     // a_ctz_32(v)  → (v as u32).trailing_zeros()
│     │     // a_clz_32(v)  → (v as u32).leading_zeros()
│     │     // a_crash()    → core::intrinsics::abort()
│     │
│     ├── core::ptr::NonNull (内部安全指针包装, 可选)
│     ├── core::mem::size_of (用于结构体大小计算)
│     └── core::usize::MAX (等价于 C 的 SIZE_MAX)
│
└── 递归依赖终止说明
      ├── atomic 操作 → 使用 Rust core::sync::atomic 标准库, 无需额外 spec
      ├── 系统调用 (brk/mmap/mprotect/munmap/madvise) → rusl 通过 asm! 自行封装
      ├── 锁原语 → rusl 自行实现自旋锁 (基于 AtomicBool + futex 或纯 spin)
      ├── get_random_secret → rusl 自行实现 (asm! 读 auxv 或 rdrand)
      ├── nontrivial_free / free_group / okay_to_free → 来自 free.rs, 其规约见 free.md Rust spec
      └── alloc_meta 的完整规约 → 见本文件 [内部函数规约] 部分
```

---

## 模块结构 (Rust 重新设计)

原 C 代码集中在 `malloc.c` 中。Rust 版本建议按以下模块划分:

```
src/malloc/mallocng/
├── mod.rs          → 模块入口, 声明子模块, re-export pub(crate) 符号
├── meta.rs         → Meta / Group / MetaArea / MallocContext 结构体
│                     + 所有原 meta.h inline 函数 (pub(crate))
├── glue.rs         → 锁原语 / 随机密钥 / 系统调用封装 (pub(crate))
├── malloc.rs       → malloc 核心实现 (本 spec 主体)
│   pub unsafe extern "C" fn malloc(n: usize) -> *mut c_void;
│   pub(crate) unsafe fn alloc_meta() -> *mut Meta;
│   pub(crate) unsafe fn is_allzero(p: *const u8) -> bool;
│   static SIZE_CLASSES: [u16; 48];
│   static mut CTX: MallocContext;
├── free.rs         → free 核心实现
├── realloc.rs      → realloc 核心实现
├── aligned_alloc.rs → aligned_alloc 实现
├── donate.rs       → donate 实现
└── malloc_usable_size.rs → malloc_usable_size 实现
```

---

## [GUARANTEE]

### 对外导出接口

```rust
// [Visibility]: Public — POSIX 标准函数, <stdlib.h> 声明
// [ABI Compatibility]: extern "C", 参数布局与返回值布局与原 C 接口完全兼容
// [符号名稳定性]: #[no_mangle] 确保编译后符号名为 "malloc"
#[no_mangle]
pub unsafe extern "C" fn malloc(n: usize) -> *mut core::ffi::c_void;
```

#### 意图 (Intent)

分配 `n` 字节的未初始化内存。使用分尺寸类别 (size class) 的 group 分配策略优化常见小分配的性能和内存效率。对于大分配 (>= `MMAP_THRESHOLD` = 131052 字节)，直接使用 `mmap`。

Rust 实现差异:
- 使用 `usize` 替代 C 的 `size_t` (在 64 位平台上均为 8 字节, ABI 完全兼容)
- 返回 `*mut c_void` 替代 C 的 `void *`
- 内部实现使用 Rust 安全抽象 (`Option`, `NonNull` 等)，但对外接口保持裸指针以兼容 C ABI

#### 前置条件

- 无特殊前置条件 (分配器在首次调用时延迟初始化, `CTX.init_done` 控制)
- 在多线程环境中: 调用者无需持有任何锁 (内部自动加锁)
- `n` 可以是任意 `usize` 值 (包括 0)

#### 后置条件

**Case 1 (成功)**:
- 返回指向至少 `n` 字节对齐内存的指针, 内存内容未初始化
- 返回的指针对齐到 16 字节边界 (`(p as usize) & 15 == 0`)
- 通过 `get_meta(p)` 可反向推导出所属的 `Meta` 和 `Group`

**Case 2 (`n == 0` 或溢出)**:
- 若 `n >= usize::MAX / 2 - 4096`: 设置 `errno = ENOMEM`, 返回 `core::ptr::null_mut()`

**Case 3 (内存耗尽)**:
- 返回 `core::ptr::null_mut()`, 设置 `errno = ENOMEM`

#### 系统算法 (System Algorithm)

**1. 溢出检查**: 调用 `size_overflows(n)`, 若溢出返回 `null`。

**2. 大块路径** (`n >= MMAP_THRESHOLD`):
```
needed = (n + IB + UNIT + PGSZ - 1) & !(PGSZ - 1)  // 向上对齐到页
p = __mmap(0, needed, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANON, -1, 0)
if p == MAP_FAILED => 返回 null
wrlock()
step_seq()
m = alloc_meta()                                      // 分配 Meta 对象
if m == null => unlock(); __munmap(p, needed); 返回 null  // 注: C spec 中无此路径
m.sizeclass = 63   // 大块标记
m.maplen = needed / PGSZ
m.avail_mask = 0; m.freed_mask = 0
ctx.mmap_counter += 1
ctr = ctx.mmap_counter  // 快照用于地址随机化
unlock()
return enframe(&*m, 0, n, ctr)
```

**3. 小/中块路径** (`n < MMAP_THRESHOLD`):
```
sc = size_to_class(n)
// 快速路径 (读锁)
rdlock()
// 粗粒度类别优化: 若 sc 的偶数类别无 group, 尝试使用 sc|1 的更大类别
loop {
    g = ctx.active[adjust_sc]
    if g.is_null() => break to slow_path
    mask = g.avail_mask.load(Relaxed)
    if mask == 0 => break to slow_path
    first = mask.trailing_zeros() as u32  // 替代 a_ctz_32
    if g.avail_mask.compare_exchange(mask, mask - (1u32<<first), AcqRel, Acquire).is_ok() {
        ctr = ctx.mmap_counter.load(Relaxed) // 快照 (lock-free)
        unlock()
        return enframe(&*g, first as usize, n, ctr)
    }
    // CAS 失败, 重试 (另一个线程抢先分配了此 slot)
}
// 慢速路径 (写锁)
upgradelock()  // 或 unlock(); wrlock()
idx = alloc_slot(sc, n)  // 尝试从现有组获取或创建新组
if idx < 0 => unlock(); 返回 null
ctr = ctx.mmap_counter.load(Relaxed)
unlock()
g = ctx.active[sc].as_ref().unwrap()
return enframe(g, idx as usize, n, ctr)
```

#### 并发安全

Rust 实现使用以下机制替代 C 的并发策略:

| C 原语 | Rust 替代 | 说明 |
|--------|----------|------|
| `volatile int` + `a_cas` | `AtomicI32::compare_exchange` | avail_mask/freed_mask 的 lock-free 更新 |
| `a_ctz_32` | `u32::trailing_zeros()` | 掩码 → slot 索引转换 |
| `rdlock/wrlock` | 自旋锁 (基于 `AtomicBool`) | fast-path 的乐观读取, slow-path 的排他写入 |
| `MT` 检测 | `need_locks: AtomicBool` | 单线程优化, 跳过锁操作 |
| `mmap_counter` 快照 | `AtomicU32::load(Relaxed)` | lock-free 读取, 仅用于地址随机化 |

#### 不变量

- 所有从 `malloc` 返回的指针对齐到 16 字节
- 分配的内存块前 4 字节 (IB) 为带内头部, 包含 slot 索引和保留大小
- 通过 `get_meta()` 可反向从指针推导出所属的 `Meta` 和 `Group`

---

### 内部 `pub(crate)` 符号 (被其他模块依赖)

#### `alloc_meta`

```rust
// [Visibility]: Internal — pub(crate), 被 free.rs / donate.rs 等模块调用
//   原 C: __attribute__((__visibility__("hidden"))) → 通过 glue.h 宏重命名为 __malloc_alloc_meta
pub(crate) unsafe fn alloc_meta() -> *mut Meta;
```

**意图**: 从元数据区池中分配一个新的 `Meta` 对象。首次调用时自动初始化全局上下文。若当前元数据区耗尽, 通过 `brk()` 或 `mmap()` 扩展元数据区。

**前置条件**:
- 调用者需持有写锁 (`wrlock()`)
- `CTX` 全局可访问

**后置条件**:
- **Case 1 (成功)**: 返回指向新分配 `Meta` 的指针, 其 `prev` 和 `next` 字段为 `null` (已清零)。`CTX.init_done == true`。
- **Case 2 (失败)**: 返回 `null` (`mmap` 失败或 `mprotect` 失败且 `errno != ENOSYS`)

**系统算法**:
1. 若 `!CTX.init_done`: 获取页面大小 (`PGSZ`) 和随机密钥 (`get_random_secret()`), 设置 `init_done = true`。
2. 快速路径: 从 `CTX.free_meta_head` 空闲链表队首取出 meta (`dequeue_head`)。
3. 若空闲链表为空, 检查 `CTX.avail_meta_count`:
   - 若 `avail_meta_count == 0`: 通过 `brk()` 或 `mmap()` 扩展元数据区。
   - 第一页设为 `PROT_NONE` 作为保护区。
   - 将新页链接入 `meta_area_head/tail` 链表。
   - 设置 `meta_area_tail.check = CTX.secret`。
4. `avail_meta_count -= 1`, 从 `avail_meta` 取出 meta, 清零链表指针后返回。

**Rust 设计要点**:
- `MetaArea` 使用 repr(C) 确保内存布局与 C 一致 (页内数组布局)
- 内存管理使用裸指针而非 `Box`/`Vec` (因为 Meta 分配在 mmap 页内, 不适合 Rust 标准分配器)
- 返回的裸指针由调用者负责正确使用和最终通过 `free_meta` 归还

**不变量**:
- 分配的每个 `MetaArea` 页面大小为 4096 字节
- `MetaArea.check` 始终等于 `CTX.secret`
- 若 `PGSZ < 4096`, 强制使用 4096

---

#### `is_allzero`

```rust
// [Visibility]: Internal — pub(crate), 被 calloc 模块调用
//   原 C: __attribute__((__visibility__("hidden"))) → 通过 glue.h 宏重命名为 __malloc_allzerop
pub(crate) unsafe fn is_allzero(p: *const u8) -> bool;
```

**意图**: 判断指针 `p` 指向的已分配内存块是否可以被视为全部为零, 从而在 `calloc` 实现中跳过显式的 `memset`。该优化适用于来自 `mmap` 全新分配或来自 fresh OS 页面的内存块。

**前置条件**:
- `p` 必须是 `malloc` 返回的有效指针
- `p` 对齐到 16 字节

**后置条件**:
- **Case 1 (全部为零)**: 返回 `true` — sizeclass >= 48 的大块 mmap 分配, 或 stride 小于名义尺寸 (slot 未被完全使用)
- **Case 2 (可能非零)**: 返回 `false` — 可能包含先前释放留下的脏数据

**系统算法**:
1. `g = get_meta(p)`: 获取关联的 `Meta`
2. 若 `g.sizeclass >= 48`: 返回 `true` (大块 mmap, OS 已清零)
3. 若 `get_stride(g) < UNIT * SIZE_CLASSES[g.sizeclass]`: 返回 `true` (非标准 stride)
4. 否则返回 `false`

**Rust 设计要点**:
- 返回 `bool` 替代 C 的 `int` (语义更清晰)
- 函数本身仅执行纯读操作, 但仍标记为 `unsafe` 因为 `get_meta(p)` 的指针解析需要调用者保证 `p` 有效性

---

#### 全局数据

```rust
// [Visibility]: Internal — pub(crate), 被所有 mallocng 模块引用
// 大小类别查找表
pub(crate) static SIZE_CLASSES: [u16; 48] = [
    1, 2, 3, 4, 5, 6, 7, 8,    // class 0-7:  16B-128B
    9, 10, 12, 15,              // class 8-11: 144B-240B
    18, 20, 25, 31,              // class 12-15: 288B-496B
    36, 42, 50, 63,              // class 16-19: 576B-1008B
    72, 84, 102, 127,            // class 20-23: 1152B-2032B
    146, 170, 204, 255,          // class 24-27: 2336B-4080B
    292, 340, 409, 511,          // class 28-31: 4672B-8176B
    584, 682, 818, 1023,         // class 32-35: 9344B-16368B
    1169, 1364, 1637, 2047,      // class 36-39: 18704B-32752B
    2340, 2730, 3276, 4095,      // class 40-43: 37440B-65520B
    4680, 5460, 6552, 8191,      // class 44-47: 74880B-131056B
];
// 每个元素 = UNIT 数; 实际字节数 = UNIT * SIZE_CLASSES[sc]
```

```rust
// [Visibility]: Internal — pub(crate), 被所有 mallocng 模块引用
// 全局分配器上下文
pub(crate) static CTX: MallocContext = MallocContext::new();
// OR: 使用 Once / Lazy 初始化模式 (no_std 下可自行实现 Once)
```

---

### 内部私有函数 (`static` → 模块私有)

#### `alloc_slot`

```rust
// [Visibility]: Private — 模块内可见, 不对外导出
//   原 C: static int alloc_slot(int sc, size_t req)
unsafe fn alloc_slot(sc: usize, req: usize) -> Option<usize>;
```

**意图**: 在尺寸类别 `sc` 中分配一个 slot。首先尝试从现有 group 中获取可用 slot, 若失败则创建新的分配组。

**前置条件**:
- 调用者需持有写锁 (`wrlock()`) 或升级锁 (`upgradelock()`)
- `sc < 48`

**后置条件**:
- **Case 1 (成功)**: 返回 `Some(idx)`, idx 为 slot 索引, 调用者可通过 `ctx.active[sc]` 获取对应 group
- **Case 2 (失败)**: 返回 `None` (`alloc_group` 失败)

**系统算法**:
1. 调用 `try_avail(&mut ctx.active[sc])` 尝试从现有 group 找到可用 slot
2. 若成功: 使用 `first.trailing_zeros()` 将掩码转为索引, 返回 `Some(idx)`
3. 若失败: 调用 `alloc_group(sc, req)` 创建新 group
4. 若 `alloc_group` 返回 `None`: 返回 `None`
5. 新 group: `avail_mask -= 1` (消耗首个 slot), `queue(...)` 将新 group 加入 `ctx.active[sc]`
6. 返回 `Some(0)` (新 group 的首个 slot)

**Rust 设计要点**:
- 返回 `Option<usize>` 替代 C 的 `-1` 错误哨兵值 (更符合 Rust 惯例)
- 调用者使用 `match` 或 `?` 处理 Option

---

#### `try_avail`

```rust
// [Visibility]: Private — 模块内可见
//   原 C: static uint32_t try_avail(struct meta **pm)
unsafe fn try_avail(pm: &mut *const Meta) -> u32;
```

**意图**: 从 `*pm` 指向的 group 开始, 沿着循环链表寻找包含可用 slot 的 group。若当前 group 无可用 slot, 则遍历链表、跳过完全空闲的 group、必要时激活更多 slot。

**前置条件**:
- `pm` 指向有效的 `*const Meta` (循环链表或 null)
- 调用者需持有读锁或写锁

**后置条件**:
- **Case 1 (成功)**: 返回非零 `u32` (恰好设置一位的掩码), `*pm` 更新为包含可用 slot 的 group
- **Case 2 (失败)**: 返回 0, `*pm` 可能已更改 (跳过已满的 group) 或为 null

**系统算法**:
1. 当前 group 检查: 读 `m.avail_mask`, 若非零则直接返回最低置位
2. 链表遍历:
   - 若 `avail_mask == 0 && freed_mask == 0` (全满): dequeue, 继续检查下一个
   - 若 `avail_mask == 0 && freed_mask != 0` (全满但有已释放 slot 可回收): 跳到下一个
3. 跳过完全空闲的 group (freed_mask 覆盖所有 slot 且 freeable)
4. 延迟激活: 若 freed 的 slot 全在未激活区域, 跳到下一个; 仅当链表中唯一 group 时才增加 active_idx
5. 激活 group: `activate_group(m)` 将 freed_mask 转移到 avail_mask
6. 反弹衰减: `decay_bounces(m.sizeclass)`

**Rust 设计要点**:
- 使用 `&mut *const Meta` 作为 out-parameter (升级了 C 的双指针)
- 返回 `u32` 与 C 保持一致 (位掩码, 用于 `trailing_zeros()`)
- 内部链表遍历全部通过裸指针操作

---

#### `alloc_group`

```rust
// [Visibility]: Private — 模块内可见
//   原 C: static struct meta *alloc_group(int sc, size_t req)
unsafe fn alloc_group(sc: usize, req: usize) -> Option<*mut Meta>;
```

**意图**: 为尺寸类别 `sc` 创建一个新的分配组 (`Meta` + `Group`), 确定 slot 数量, 分配存储空间 (mmap 或嵌套分配), 并初始化元数据和组头。

**前置条件**:
- 调用者需持有写锁 (`wrlock()`)
- `sc < 48`

**后置条件**:
- **Case 1 (成功)**: 返回 `Some(meta_ptr)`, 该 Meta 的 `avail_mask` 已设置所有 slot 为可用 (除首个已消耗)、`freed_mask` 清零、`mem` 指向新 Group、`last_idx` 和 `sizeclass` 已设置
- **Case 2 (失败)**: 返回 `None` (`alloc_meta` 失败或 `mmap` 失败, 已调用 `free_meta` 归还 Meta)

**系统算法**:
1. `size = UNIT * SIZE_CLASSES[sc]`
2. 确定 slot 数量:
   - sc < 9: 根据 `usage_by_class[sc]` 在 `SMALL_CNT_TAB[sc]` 的三个等级中选择
   - sc >= 9: 从 `MED_CNT_TAB[sc & 3]` 出发, 低使用量时减半
   - 若 `size*cnt >= 65536*UNIT` 继续减半 (slot 偏移不超过 16 位)
   - 若 `cnt==1 && size+UNIT <= PGSZ/2` 增大到 2
3. 大尺寸路径 (`size*cnt+UNIT > PGSZ/2`):
   - 检查反弹状态 (`is_bouncing`), 更新反弹计数 (`account_bounce`)
   - 尝试减少 cnt 控制浪费率 (不超过当前使用量的 25%)
   - 若低使用量、未反弹、cnt<=7: 尝试降级为独立 mmap (cnt=1)
   - `__mmap()` 分配整页内存
   - 计算 `active_idx`, 考虑 4KB 边界对齐
4. 小尺寸路径 (嵌套):
   - `alloc_slot(j, ...)` 在更大尺寸类别的 group 中分配空间
   - `enframe()` 初始化存储区
   - 写入特殊标记 `p[-3] = (p[-3] & 31) | (6 << 5)` (reserved=6 表示嵌套组)
   - 初始化所有 slot 的越界检查字节
5. 初始化元数据: 设置 `avail_mask`, `freed_mask`, `mem.meta`, `mem.active_idx`, `last_idx`, `freeable=true`, `sizeclass=sc`
6. 更新使用量: `ctx.usage_by_class[sc] += cnt`

**Rust 设计要点**:
- 返回 `Option<*mut Meta>` 替代 C 的 `NULL` 哨兵
- 内部使用 `NonNull` 或裸指针管理分配的内存
- `__mmap` 的结果使用 `is_null()` 检测失败 (替代 C 的 `MAP_FAILED`)

---

## 全局不变量 (Global Invariants)

适用于 mallocng 分配器的整个生命周期, 与 C spec 一致:

1. **元数据完整性**: 每个 `MetaArea.check` 必须等于 `CTX.secret`。通过 `get_meta()` 验证时检查此条件。

2. **Group-元数据双向链接**: 对于活跃 group, `g.mem.as_ref().unwrap().meta == g` 始终成立。

3. **Slot 索引范围**: 任何已分配 slot 的索引 `idx` 满足 `idx <= meta.last_idx`。

4. **位掩码一致性**: 对于任意 group, slot 索引 `i` 不可能同时在 `avail_mask` 和 `freed_mask` 中, 即 `!(avail_mask & (1u32 << i) & freed_mask)`。

5. **尺寸类别范围**: `meta.sizeclass` 的取值范围为 0-47 (常规分配) 或 63 (大块 mmap 分配)。

6. **锁层级**: 读锁和写锁在 `RDLOCK_IS_EXCLUSIVE == 1` 时实现相同 (均为排他锁)。协议保留了读/写锁的语义以供未来优化。Rust 实现中可使用 `Mutex` 或自旋锁统一管理。

7. **Active 链表**: `ctx.active[sc]` 是循环双向链表, 或为 `null` (空链表)。链表中的每个 group 至少有一个 `avail_mask` 或 `freed_mask` 中的可用 slot。

8. **使用量统计**: `ctx.usage_by_class[sc]` 等于该尺寸类别所有活跃 group 中 `last_idx + 1` 之和。

---

## 内部依赖符号汇总

| 符号 | Rust 类型/表示 | 来源模块 | 可见性 |
|------|---------------|---------|--------|
| `malloc` | `unsafe extern "C" fn(usize) -> *mut c_void` | malloc.rs | **Public** `<stdlib.h>` |
| `alloc_meta` | `pub(crate) unsafe fn() -> *mut Meta` | malloc.rs | **Internal** (被 free/donate 引用) |
| `is_allzero` | `pub(crate) unsafe fn(*const u8) -> bool` | malloc.rs | **Internal** (被 calloc 引用) |
| `SIZE_CLASSES` | `pub(crate) static [u16; 48]` | malloc.rs | **Internal** (被所有模块引用) |
| `CTX` | `pub(crate) static MallocContext` | malloc.rs | **Internal** (被所有模块引用) |
| `SMALL_CNT_TAB` | `static [[u8; 3]; 9]` | malloc.rs | **Private** |
| `MED_CNT_TAB` | `static [u8; 4]` | malloc.rs | **Private** |
| `alloc_slot` | `unsafe fn(usize, usize) -> Option<usize>` | malloc.rs | **Private** |
| `try_avail` | `unsafe fn(&mut *const Meta) -> u32` | malloc.rs | **Private** |
| `alloc_group` | `unsafe fn(usize, usize) -> Option<*mut Meta>` | malloc.rs | **Private** |
| `Meta` / `Group` / `MetaArea` / `MallocContext` | 结构体 | meta.rs | **Internal** pub(crate) |
| `UNIT` / `IB` / `MMAP_THRESHOLD` | const | meta.rs | **Internal** pub(crate) |
| `get_meta` / `get_slot_index` / `get_stride` 等 | pub(crate) fn | meta.rs | **Internal** |
| `rdlock` / `wrlock` / `unlock` / `upgradelock` | pub(crate) fn | glue.rs | **Internal** |
| `get_random_secret` | pub(crate) fn | glue.rs | **Internal** |
| `__brk` / `__mmap` / `__mprotect` / `__munmap` | pub(crate) unsafe fn | glue.rs (syscall) | **Internal** |
| `a_cas` / `a_ctz_32` / `a_or` / `a_crash` / `a_clz_32` | `core::sync::atomic` / `u32` intrinsic | Rust core | 标准库, 无需额外 spec |
| `errno` / `ENOMEM` | 全局变量 / 常量 | 错误处理模块 | **Public** |

---

## 跨文件依赖说明

| 依赖符号 | 来源文件 (Rust) | 来源文件 (C) | 说明 |
|---------|----------------|-------------|------|
| `Meta` / `Group` / `MetaArea` / `MallocContext` | `meta.rs` | `meta.h` | 核心数据结构, 重新设计为 repr(C) Rust struct |
| `UNIT` / `IB` / `MMAP_THRESHOLD` / `PGSZ` | `meta.rs` | `meta.h` | 编译期常量 |
| `size_to_class()` / `size_overflows()` | `meta.rs` | `meta.h` | 大小分类和内联辅助 |
| `enframe()` / `get_meta()` / `get_stride()` / `set_size()` | `meta.rs` | `meta.h` | 槽位操作 |
| `activate_group()` / `queue()` / `dequeue()` / `dequeue_head()` | `meta.rs` | `meta.h` | 链表和位掩码操作 |
| `free_meta()` / `step_seq()` / `record_seq()` | `meta.rs` | `meta.h` | 生命周期和序列号 |
| `account_bounce()` / `decay_bounces()` / `is_bouncing()` | `meta.rs` | `meta.h` | 反碎片化控制 |
| `rdlock()` / `wrlock()` / `unlock()` / `upgradelock()` | `glue.rs` | `glue.h` | 锁操作 |
| `get_random_secret()` | `glue.rs` | `glue.h` | 随机密钥生成 |
| `__brk()` / `__mmap()` / `__mprotect()` / `__munmap()` | `glue.rs` (via syscall) | `glue.h` → syscall | 系统调用封装 |
| `nontrivial_free()` | `free.rs` | `free.c` | 被 alloc_group 在嵌套组场景下递归调用 |
| `a_cas` / `a_ctz_32` / `a_or` / `a_crash` / `a_clz_32` | `core::sync::atomic` | `atomic.h` | 原子操作, 由 Rust 标准库直接替代 |

---

## rusl no_std 适配说明

1. **无 `libc` crate**: 所有 C ABI 类型使用 `core::ffi::c_void`、`usize` (等价 `size_t`)、`i32` (等价 `c_int`)、`u32` / `u64` 等 Rust 原生类型。`extern "C"` 声明确保 ABI 兼容。

2. **no_std 约束**: 不依赖 `std::alloc`。`malloc` 自身即为分配器实现, 无需依赖 Rust 全局分配器。`core::ptr::null_mut()` 替代 `std::ptr::null_mut()`; `core::sync::atomic::*` 替代 `std::sync::atomic::*`。

3. **系统调用由 asm! 直接发起**: `__brk` / `__mmap` / `__mprotect` / `__munmap` / `__madvise` / `__mremap` 全部通过 `core::arch::asm!` 内联汇编发起 syscall 指令, 不经过任何外部 FFI 封装层。

4. **原子操作替代**:
   - `a_cas(p, t, s)` → `(*p).compare_exchange(t, s, AcqRel, Acquire)`
   - `a_or(p, v)` → `(*p).fetch_or(v, AcqRel)`
   - `a_ctz_32(v)` → `v.trailing_zeros()`
   - `a_clz_32(v)` → `v.leading_zeros()`
   - `a_crash()` → `core::intrinsics::abort()`

5. **锁实现**: rusl 不能依赖 `std::sync::Mutex`。需自行实现基于 `AtomicBool` 的自旋锁 (可使用 `spin` crate 的 `no_std` 兼容版本, 或直接实现 `compare_exchange` 循环)。

6. **随机密钥**: `get_random_secret()` 需 rusl 自行实现。可通过以下方式获取熵:
   - 读取 auxv 中的 `AT_RANDOM` (需动态链接器或 boot 阶段设置)
   - 使用 `RDRAND` 指令 (x86_64) 或架构对应的硬件随机数指令
   - 栈地址作为基础熵源 (与 C spec 一致)

7. **`errno` 机制**: rusl 需自行实现 thread-local `errno` 存储 (使用 `#[thread_local]` 或 OS 提供的 TLS 机制) 及 `ENOMEM` 常量定义。

8. **结构体内存布局**: `Meta` / `Group` / `MetaArea` / `MallocContext` 使用 `#[repr(C)]` 确保内存布局与 C 完全一致。位域字段 (`last_idx:5`, `freeable:1`, `sizeclass:6`, `maplen`) 在 Rust 中展开为普通字段 + 访问器方法, 通过位运算实现编解码。

9. **未使用符号省略**: C spec 中标记为 `[Visibility]: Internal` 且 rusl 不需要的符号 (如 `a_crash` 仅在 `a_cas`/`a_ctz_32` 等被 Rust core 完全替代后不再需要; 如调试辅助 `dump_heap`) 可以从 Rust spec 中省略。

---

## 递归依赖追踪终止说明

以下依赖已在外部模块独立描述, 本 spec 仅声明依赖关系:

- `nontrivial_free()` / `free_group()` / `okay_to_free()`: 来自 `free.rs`, 完整规约见 `src/malloc/mallocng/rust-spec/free.md`
- `Meta` / `Group` / `MetaArea` / `MallocContext` 结构体及所有 inline 函数的完整字段级规约: 见 `src/malloc/mallocng/rust-spec/meta.md`
- 锁原语 / 随机密钥 / 系统调用封装的完整规约: 见 `src/malloc/mallocng/rust-spec/glue.md`
- 原子操作 (`a_cas`/`a_ctz_32`/`a_or`/`a_crash`/`a_clz_32`): 由 Rust `core` 标准库直接提供, 无需额外 spec
- `errno`/`ENOMEM`: POSIX 标准机制, rusl 需自行实现, 不在 mallocng 范围内