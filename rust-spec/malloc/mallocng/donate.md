# donate Rust 接口规约

> C 源文件: `src/malloc/mallocng/donate.c`
> 对应 C spec: `src/malloc/mallocng/spec/donate.md`
> 本文件将 C spec 按 Rust 设计哲学重新设计，对外导出符号保持 ABI 兼容，内部实现可完全重新设计。

---

## 递归依赖追踪（完整符号依赖图）

```
__malloc_donate (对外导出, extern "C")
  │
  └── donate (内部函数, pub(crate))
        ├── [外部] core::ptr::write_bytes(base, 0, len)
        │     // Rust no_std 替代 C 的 memset, 无需外部 crate
        │
        ├── crate::malloc::alloc_meta() -> Option<NonNull<Meta>>
        │     // 元数据分配器, 定义于 malloc 模块
        │     // 等效于 C 的 alloc_meta(), 隐藏可见性符号
        │     │
        │     ├── [依赖] crate::malloc::meta::dequeue_head()
        │     │     // 从空闲 meta 链表头部取出 meta
        │     │     └── crate::malloc::meta::dequeue()
        │     │           // 从循环双向链表中移除节点
        │     │
        │     ├── [依赖] crate::malloc::CTX (全局上下文)
        │     │     // struct MallocContext 提供 init_done, secret,
        │     │     // free_meta_head, avail_meta, meta_area_head 等字段
        │     │
        │     ├── [依赖] crate::malloc::glue::page_size()
        │     │     // 获取运行时页大小
        │     │
        │     ├── [依赖] crate::malloc::glue::random_secret()
        │     │     // 生成 64 位随机密钥
        │     │
        │     ├── [依赖] brk() / mmap() / mprotect() 系统调用
        │     │     // rusl 通过 asm! 内联汇编直接发起 syscall
        │     │     // mmap: SYS_mmap (9), PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANON
        │     │     // brk: SYS_brk (12)
        │     │     // mprotect: SYS_mprotect (10)
        │     │
        │     ├── [依赖] crate::malloc::meta::MetaArea
        │     │     // 元数据页结构体, 包含 check/next/nslots/slots[]
        │     │
        │     └── [递归终止] dequeue → 循环链表操作, O(1), 无进一步依赖
        │
        ├── crate::malloc::meta::queue(head: &mut Option<NonNull<Meta>>, m: NonNull<Meta>)
        │     // 将 Meta 节点插入循环双向链表尾部
        │     // 来自 meta 模块 (原 meta.h static inline)
        │     // 无进一步依赖
        │
        ├── crate::malloc::CTX: MallocContext
        │     // 全局分配器上下文静态变量
        │     // 来自 malloc 模块 (原 extern struct malloc_context ctx)
        │     │
        │     ├── [依赖] crate::malloc::meta::MallocContext
        │     │     // 结构体定义: secret, init_done, active[48],
        │     │     // free_meta_head, avail_meta, meta_area_head 等字段
        │     │
        │     └── [关键字段] active: [Option<NonNull<Meta>>; 48]
        │           // 48 个 size class 的活跃 meta 循环链表头
        │           // donate 仅写入 ctx.active[sc], 读取 ctx.init_done
        │
        ├── crate::malloc::SIZE_CLASSES: [u16; 48]
        │     // 大小类别查找表 (以 UNIT 为单位)
        │     // 来自 malloc 模块 (原 const uint16_t size_classes[])
        │     // 编译期常量, 无进一步依赖
        │
        ├── crate::malloc::meta::Meta
        │     // 内存组元数据结构体 (原 struct meta)
        │     │
        │     ├── 字段: prev, next (双向循环链表指针)
        │     ├── 字段: mem (指向关联 Group)
        │     ├── 字段: avail_mask, freed_mask (位掩码, volatile)
        │     ├── 字段: last_idx:5, freeable:1, sizeclass:6, maplen
        │     └── [不变量] meta.mem.meta == meta (双向绑定)
        │
        ├── crate::malloc::meta::Group
        │     // 内存组结构体 (原 struct group)
        │     │
        │     ├── 字段: meta (指向 Meta)
        │     ├── 字段: active_idx:5
        │     ├── 字段: pad[N] (填充至 UNIT 字节)
        │     ├── 字段: storage[] (柔性数组, 实际存储区域)
        │     └── [不变量] group 起始地址为页对齐 (mmap 分配时)
        │
        └── crate::malloc::meta::UNIT: usize = 16
              // 最小分配单位和对齐粒度
              // 编译期常量, 无进一步依赖
```

---

## [RELY]

### 依赖 1: Meta 结构体 (内部类型, crate::malloc::meta)

```rust
// [Visibility]: Internal — musl/rusl mallocng 内部元数据结构, 不对外导出
// [Layout]: #[repr(C)] 或紧凑 Rust 结构体 (rusl 内部可自由设计)
// 原 C 类型: struct meta (定义于 meta.h)
pub(crate) struct Meta {
    pub(crate) prev: Option<NonNull<Meta>>,  // 双向循环链表前驱
    pub(crate) next: Option<NonNull<Meta>>,  // 双向循环链表后继
    pub(crate) mem: Option<NonNull<Group>>,  // 指向关联的 group
    pub(crate) avail_mask: AtomicI32,        // 可用槽位位掩码 (volatile → AtomicI32)
    pub(crate) freed_mask: AtomicI32,        // 已释放槽位位掩码
    pub(crate) last_idx: u8,                 // 最大槽位索引 (0..31, 5 bits)
    pub(crate) freeable: bool,               // 是否可被整体释放 (1 bit)
    pub(crate) sizeclass: u8,                // 大小类别 (0..47 或 63, 6 bits)
    pub(crate) maplen: usize,                // mmap 映射页数 (0 表示非 mmap 分配)
}
// 注: 原 C 使用位域打包 (last_idx:5, freeable:1, sizeclass:6, maplen:N),
// rusl 内部可用独立的 u8/bool/usize 字段简化设计, 无需严格按位域布局.
// 若需 C ABI 互操作 (跨语言边界传递), 则必须使用 #[repr(C)] 并精确匹配位域.
```

### 依赖 2: Group 结构体 (内部类型, crate::malloc::meta)

```rust
// [Visibility]: Internal — musl/rusl mallocng 内部数据结构, 不对外导出
// [Layout]: 若通过 mmap 分配则需页对齐; 内部结构可自由设计
// 原 C 类型: struct group (定义于 meta.h)
#[repr(C)]  // 需要精确控制内存布局 (内嵌于分配的原始内存区域)
pub(crate) struct Group {
    pub(crate) meta: *mut Meta,       // 指向本组元数据的反向指针
    pub(crate) active_idx: u8,        // 当前活动掩码最高位编号 (0..31, 5 bits)
    pub(crate) pad: [u8; 11],         // 填充至 16 字节 (UNIT - sizeof(*mut Meta) - 1)
    // storage[] 是柔性数组, Rust 中用 DST 或裸指针算术访问
}
// 注: storage[] 柔性数组在 Rust 中无法直接表示为 struct 字段.
// rusl 使用方式: g 为页对齐的 *mut Group, 通过 g.add(1) 获取 storage 起始地址,
// 再按 stride 偏移访问各个 slot. 这要求 Group 大小精确为 UNIT 字节.
```

### 依赖 3: UNIT 常量 (内部常量, crate::malloc::meta)

```rust
// [Visibility]: Internal — 不对外导出
// 原 C: #define UNIT 16
pub(crate) const UNIT: usize = 16;
```

### 依赖 4: queue 函数 (内部函数, crate::malloc::meta)

```rust
// [Visibility]: Internal — 不对外导出, pub(crate)
// 原 C: static inline void queue(struct meta **phead, struct meta *m)
// 意图: 将 meta 节点插入双向循环链表尾部

/// 将 meta 节点插入以 `head` 为头指针的循环双向链表.
///
/// # 前置条件
/// - `head` 指向一个有效的链表头指针 (可能为 None 表示空链表)
/// - `m` 是一个有效的 Meta 指针, 且其 prev/next 均为 None (当前不在任何链表中)
///
/// # 后置条件
/// - Case 1 (空链表 head.is_none()):
///   m.prev = m; m.next = m; *head = m; // 自环
/// - Case 2 (非空链表):
///   m 被插入到 *head 之前 (循环链表尾部)
///   m.prev 指向旧尾部, m.next 指向 *head
///   循环链表完整性保持
///
/// # 复杂度
/// O(1)
pub(crate) unsafe fn queue(head: &mut Option<NonNull<Meta>>, m: NonNull<Meta>);
```

### 依赖 5: SIZE_CLASSES 常量数组 (内部数组, crate::malloc)

```rust
// [Visibility]: Internal — 不对外导出, pub(crate)
// 原 C: const uint16_t size_classes[48] (定义于 malloc.c)
// 每个元素表示该 size class 一个 slot 占用的 UNIT 数
pub(crate) const SIZE_CLASSES: [u16; 48] = [
    1, 2, 3, 4, 5, 6, 7, 8,       // class 0-7:  16B-128B
    9, 10, 12, 15,                  // class 8-11: 144B-240B
    18, 20, 25, 31,                 // class 12-15: 288B-496B
    36, 42, 50, 63,                 // class 16-19: 576B-1008B
    72, 84, 102, 127,               // class 20-23: 1152B-2032B
    146, 170, 204, 255,             // class 24-27: 2336B-4080B
    292, 340, 409, 511,             // class 28-31: 4672B-8176B
    584, 682, 818, 1023,            // class 32-35: 9344B-16368B
    1169, 1364, 1637, 2047,         // class 36-39: 18704B-32752B
    2340, 2730, 3276, 4095,         // class 40-43: 37440B-65520B
    4680, 5460, 6552, 8191,         // class 44-47: 74880B-131056B
];
```

### 依赖 6: CTX 全局上下文 (内部静态变量, crate::malloc)

```rust
// [Visibility]: Internal — 不对外导出, pub(crate)
// 原 C: struct malloc_context ctx = { 0 } (定义于 malloc.c)
// 全局唯一的 musl/rusl mallocng 分配器上下文

#[derive(Debug)]
pub(crate) struct MallocContext {
    pub(crate) secret: u64,                              // 运行时随机密钥
    #[cfg(not(defined(PAGESIZE)))]
    pub(crate) pagesize: usize,                          // 运行时页大小
    pub(crate) init_done: bool,                          // 初始化完成标志
    pub(crate) mmap_counter: u32,                        // mmap 调用计数器
    pub(crate) free_meta_head: Option<NonNull<Meta>>,    // 空闲 meta 链表头
    pub(crate) avail_meta: Option<NonNull<Meta>>,        // 可用 meta 区域起始
    pub(crate) avail_meta_count: usize,                  // 可用 meta 计数
    pub(crate) avail_meta_area_count: usize,             // 可用 meta_area 计数
    pub(crate) meta_alloc_shift: usize,                  // meta 分配指数增长因子
    pub(crate) meta_area_head: Option<NonNull<MetaArea>>, // meta_area 链表头
    pub(crate) meta_area_tail: Option<NonNull<MetaArea>>, // meta_area 链表尾
    pub(crate) avail_meta_areas: Option<NonNull<u8>>,    // 可用 meta_area 位图
    pub(crate) active: [Option<NonNull<Meta>>; 48],      // 每个 sizeclass 的活跃链表
    pub(crate) usage_by_class: [usize; 48],              // 每个 sizeclass 累计使用量
    pub(crate) unmap_seq: [u8; 32],                      // 各 size class 末次 unmap 序列号
    pub(crate) bounces: [u8; 32],                        // 弹跳计数
    pub(crate) seq: u8,                                  // 全局操作序列号 (1..255)
    pub(crate) brk: usize,                               // 当前 brk 值
}

// 全局静态实例, 初始全零
// rusl no_std 下使用 static mut + 内部可变性 (UnsafeCell/Mutex) 或裸指针管理
// 原 C 中通过 __malloc_lock 保证线程安全; rusl 采用相同策略
pub(crate) static CTX: MallocContext = MallocContext { /* 全零初始化 */ };
// 注: 实际实现中 CTX 需封装在内部可变性容器中 (如 SpinMutex<MallocContext>),
// 或维持 static mut 并用 unsafe 块访问 (与 C 风格一致).
// 此处仅描述规约, 不约束具体实现方式.
```

### 依赖 7: alloc_meta 函数 (内部函数, crate::malloc)

```rust
// [Visibility]: Internal — 不对外导出, pub(crate)
// 原 C: struct meta *alloc_meta(void), __attribute__((__visibility__("hidden")))
// 定义于 malloc.c

/// 分配一个新的 Meta 对象.
///
/// # 前置条件
/// - 调用者持有写锁
/// - CTX 全局可访问
///
/// # 后置条件
/// - Case 1 (成功): 返回 Some(meta_ptr), meta.prev 和 meta.next 已清零
/// - Case 2 (失败): 返回 None (mmap 失败且 errno != ENOSYS 时)
///
/// # 系统算法
/// 1. 若 CTX.init_done == false, 初始化: 获取页大小、生成随机密钥
/// 2. 快速路径: 从 CTX.free_meta_head 空闲链表头部取出 (dequeue_head)
/// 3. 慢速路径 (空闲链表为空):
///    a. 尝试 brk() 扩展堆顶获取新 meta_area 页
///    b. 若 brk 失败/不可用, mmap() 分配新页 (首页 PROT_NONE 保护)
///    c. 链接入 meta_area_head/tail 链表
///    d. 设置 meta_area.check = CTX.secret
/// 4. CTX.avail_meta_count -= 1, 返回 avail_meta 当前指针
///
/// # 复杂度
/// - 快速路径: O(1)
/// - 慢速路径: O(1) + 可能的一次系统调用
pub(crate) fn alloc_meta() -> Option<NonNull<Meta>>;
```

### 依赖 8: extern 类型和常量

```rust
// Rust 原生类型替代 C 类型 (no_std 环境下无需 libc crate)
use core::ffi::c_char;     // 等价于 C 的 char
use core::ffi::c_void;     // 等价于 C 的 void
use core::ptr::NonNull;    // 等价于非空裸指针语义
use core::sync::atomic::AtomicI32;  // 等价于 C volatile int (带原子语义)

// 外部系统调用 (rusl 通过 asm! 内联汇编直接发起, 不经过 libc)
// donate 本身不直接调用 syscall, 但 alloc_meta 内部调用:
//   SYS_brk (12), SYS_mmap (9), SYS_mprotect (10)
// 这些系统调用的封装函数规约见 crate::malloc::glue 模块.
```

### 依赖 9: MetaArea 结构体 (内部类型, crate::malloc::meta)

```rust
// [Visibility]: Internal — 不对外导出
// 原 C: struct meta_area (定义于 meta.h)
// 按页对齐的元数据分配区域

#[repr(C)]  // 需要页对齐和精确内存布局
pub(crate) struct MetaArea {
    pub(crate) check: u64,          // 完整性校验值 (= CTX.secret)
    pub(crate) next: *mut MetaArea, // 链表指针
    pub(crate) nslots: i32,         // 槽位数量
    // slots[] 柔性数组
}
// 不变量:
// - (area as usize) & 4095 == 0 (页对齐)
// - area.check == CTX.secret
// - 每个 meta_area 占据恰好一页 (4096 字节)
```

---

## [GUARANTEE]

### 对外导出接口

```rust
// [Visibility]: Internal — musl/rusl 内部接口, 声明于 dynlink.h, hidden 可见性
//                 仅供动态链接器 ldso/dynlink.c:reclaim() 调用
//                 将共享库可写段之间的对齐间隙内存"捐献"给 malloc 堆
//                 POSIX/C 标准未定义, 用户程序不应调用
// [ABI Compatibility]: extern "C", 参数布局与原 C 接口完全兼容
#[no_mangle]
pub unsafe extern "C" fn __malloc_donate(start: *mut c_char, end: *mut c_char);
```

### 内部辅助函数

```rust
// [Visibility]: Internal — pub(crate), 不对外导出
// 原 C: static void donate(unsigned char *base, size_t len)
// 在 Rust 中可重新设计为更安全的接口, 但保留等价的函数签名

/// 将一段已清零的连续内存区域拆分为多个大小类的单槽内存组,
/// 并将它们逐个加入全局分配器上下文 CTX.active[] 链表.
///
/// # 参数
/// - `base`: 捐献内存区域的起始地址
/// - `len`: 捐献内存区域的字节长度
///
/// # 前置条件
/// - `base` 非空
/// - `len > 0`
/// - `[base, base+len)` 所在内存页为可读写 (PROT_READ | PROT_WRITE)
/// - `CTX.init_done == true` (全局分配器上下文已初始化)
/// - `alloc_meta()` 必须能成功分配 (即存在可用的 meta 区域或能通过 brk/mmap 扩展)
/// - 调用方持有 malloc 写入锁, 或此时为单线程环境
///
/// # 后置条件
/// - `[base, base+len)` 范围内的所有字节被清零
/// - 在可用空间内, 从大到小依次建立了若干单槽 groups
/// - 每个 group 被链表化到 CTX.active[sc] 上
/// - 每个被捐献 group: freed_mask=1, avail_mask=0, freeable=false, maplen=0
/// - 遍历结束后, 未使用的尾部碎片被丢弃
///
/// # 系统算法 (从大到小贪心拆分)
/// 1. 对齐边界: base 向上对齐到 UNIT, end 向下对齐到 UNIT
/// 2. 全区域清零: core::ptr::write_bytes(base, 0, len)
/// 3. 逆序遍历大小类 (47, 43, 39, 35, 31, 27, 23, 19, 15, 11, 7, 3):
///    对每个 size class sc:
///    - 若剩余空间不足 (SIZE_CLASSES[sc] + 1) * UNIT, 跳过
///    - alloc_meta() 分配一个 Meta
///    - 将 group 起始地址作为 *mut Group, 初始化元数据和 slot 内部结构
///    - queue(&mut CTX.active[sc], meta) 加入循环双向链表
///    - 推进指针: a += (SIZE_CLASSES[sc] + 1) * UNIT
/// 4. 剩余碎片丢弃
///
/// # 单槽 Group 初始化细节
/// 对于每个被捐献的 group (仅含 1 个 slot, last_idx = 0):
/// ```text
/// meta.avail_mask = 0          // 无可用 slot (等待 free 后产生)
/// meta.freed_mask = 1          // slot 0 标记为已释放
/// meta.mem = group_ptr         // group 首地址
/// (*group_ptr).meta = meta_ptr // 反向指针
/// meta.last_idx = 0            // 仅有 slot 0
/// meta.freeable = false        // 捐献内存不可回收
/// meta.sizeclass = sc          // 绑定大小类
/// meta.maplen = 0              // 非 mmap 分配
/// ```
///
/// Slot header 字节 (位于 slot 用户数据起始位置的前 4 字节):
/// | 偏移     | 字节  | 含义 |
/// |----------|-------|------|
/// | UNIT-4   | 0     | check byte (无扩展 offset) |
/// | UNIT-3   | 255   | header: idx=31, reserved=7 |
/// | UNIT-2   | 0     | offset 低字节 (由清零初始化) |
/// | UNIT-1   | 0     | offset 高字节 (由清零初始化) |
///
/// Slot 结束标记: storage[SIZE_CLASSES[sc] * UNIT - 4] = 0
///
/// # 不变量
/// - 每个被创建的 group: meta.mem.meta == meta (双向绑定)
/// - 每个被创建的 group: meta.last_idx == 0 (单槽)
/// - 捐献内存: freeable = 0 (确保 free() 不会 munmap/madvise 这些页)
/// - maplen = 0 (确保 get_stride() 使用 UNIT * SIZE_CLASSES[sc])
///
/// # 性能特性
/// - 时间复杂度: O(N), N 为可容纳的 group 数量上限
/// - 空间开销: 每个 group 一个 Meta (典型 32 字节) + 一个 UNIT (16 字节) group header
pub(crate) unsafe fn donate(base: *mut u8, len: usize);
```

---

## 内部依赖符号汇总

| 符号 | Rust 类型/表示 | 来源模块 | 可见性 |
|------|---------------|---------|--------|
| `__malloc_donate` | `extern "C" fn(*mut c_char, *mut c_char)` | donate 模块 | **Internal** (hidden) |
| `donate` | `pub(crate) unsafe fn(*mut u8, usize)` | donate 模块 | **Internal** (pub(crate)) |
| `alloc_meta` | `pub(crate) fn() -> Option<NonNull<Meta>>` | malloc 模块 | Internal |
| `queue` | `pub(crate) unsafe fn(&mut Option<NonNull<Meta>>, NonNull<Meta>)` | meta 模块 | Internal |
| `Meta` | `pub(crate) struct Meta` | meta 模块 | Internal |
| `Group` | `#[repr(C)] pub(crate) struct Group` | meta 模块 | Internal |
| `MetaArea` | `#[repr(C)] pub(crate) struct MetaArea` | meta 模块 | Internal |
| `MallocContext` | `pub(crate) struct MallocContext` | meta/malloc 模块 | Internal |
| `CTX` | `pub(crate) static CTX: MallocContext` | malloc 模块 | Internal |
| `SIZE_CLASSES` | `pub(crate) const [u16; 48]` | malloc 模块 | Internal |
| `UNIT` | `pub(crate) const usize = 16` | meta 模块 | Internal |
| `IB` | `pub(crate) const usize = 4` | meta 模块 | Internal |
| `dequeue` | `pub(crate) unsafe fn(...)` | meta 模块 | Internal |
| `dequeue_head` | `pub(crate) unsafe fn(...)` | meta 模块 | Internal |
| `page_size` | `pub(crate) fn() -> usize` | glue 模块 | Internal |
| `random_secret` | `pub(crate) fn() -> u64` | glue 模块 | Internal |

---

## 跨文件依赖说明

| 依赖符号 | 来源文件 | 说明 |
|---------|---------|------|
| `Meta` / `Group` / `MetaArea` / `MallocContext` | `meta.rs` (mallocng) | 核心数据结构定义 |
| `UNIT` / `IB` | `meta.rs` (mallocng) | 核心常量 |
| `queue()` / `dequeue()` / `dequeue_head()` | `meta.rs` (mallocng) | 循环双向链表操作 |
| `alloc_meta()` | `malloc.rs` (mallocng) | 元数据分配函数 |
| `SIZE_CLASSES[]` | `malloc.rs` (mallocng) | 大小类别查找表 |
| `CTX` (MallocContext 全局实例) | `malloc.rs` (mallocng) | 全局分配器上下文 |
| `page_size()` / `random_secret()` | `glue.rs` (mallocng) | 平台适配层 |
| `brk()` / `mmap()` / `mprotect()` | `glue.rs` (mallocng) | 系统调用封装 (通过 `asm!`) |
| `core::ptr::write_bytes` | Rust `core` | 替代 C 的 `memset` |
| `c_char` / `c_void` / `NonNull` | Rust `core::ffi` / `core::ptr` | Rust no_std C ABI 互操作类型 |

---

## rusl no_std 适配说明

1. **无 `libc` crate**: 所有 C ABI 类型使用 `core::ffi::c_char`、`core::ffi::c_void`、`usize`（等价 `size_t`）、`u8`（等价 `unsigned char`）等 Rust 原生类型。`NonNull<Meta>` 用于表示非空指针语义。

2. **no_std 约束**: 不依赖 `std::alloc`，内部使用 mallocng 自己的分配器。`core::ptr::write_bytes` 替代 `memset`；`core::sync::atomic::AtomicI32` 替代 C `volatile int`。

3. **`memset` 替代**: 原 C 代码调用 `memset(base, 0, len)` 清零捐献区域。在 rusl `no_std` 环境下，使用 `core::ptr::write_bytes(base, 0, len)` 的实现同等效果（均为逐字节写入 0），无需依赖外部 libc。

4. **系统调用**: `alloc_meta()` 内部调用的 `brk()`、`mmap()`、`mprotect()` 等 syscall，由 rusl 通过 `asm!` 内联汇编直接发起，不经过任何外部 libc FFI 封装。`donate` 本身不直接发起 syscall。

5. **`uintptr_t` 等价**: Rust 的 `usize` 等价于 C 的 `uintptr_t`，均为指针宽度无符号整数。

6. **`size_t` 等价**: Rust 的 `usize` 等价于 C 的 `size_t`，均为地址空间最大对象大小类型。

7. **全局可变状态**: C 中的 `static struct malloc_context ctx` 是全局可变状态。rusl 需使用 `static mut` 结合内部锁 (`__malloc_lock`)，或使用 `UnsafeCell` + `Mutex` 封装。具体实现方式由 malloc 模块决定，donate 模块仅通过 `pub(crate)` 接口访问。

8. **柔性数组成员**: C 的 `struct group { ... unsigned char storage[]; }` 和 `struct meta_area { ... struct meta slots[]; }` 两个结构体均包含柔性数组。在 Rust 中无法直接表示，需通过指针算术计算偏移量访问。`Group` 和 `MetaArea` 头部使用 `#[repr(C)]` 固定布局，`storage[]`/`slots[]` 区域通过 `ptr.add(header_size)` 计算起始地址。

---

## donate 与 alloc_slot/alloc_group 的关键差异

`donate` 创建的 group 与 `alloc_group` (malloc 慢速路径) 创建的 group 有以下关键差异：

| 属性 | `donate` (捐献) | `alloc_group` (常规分配) |
|------|----------------|------------------------|
| `freed_mask` | 初始设为 1 (slot 0 标记已释放) | 初始设为 0 (所有 slot 待分配) |
| `avail_mask` | 初始设为 0 (等待 free 激活) | 初始设为首个 slot 可用 |
| `freeable` | 0 (不可回收, 捐献内存并非由 mmap 分配) | 1 (可回收) |
| `maplen` | 0 (非 mmap 分配) | 0 (嵌套) 或 >0 (独立 mmap) |
| 加入 active 链表 | 通过 `queue()` 直接加入 | 通过 `queue()` 加入 (alloc_slot 中) |
| `usage_by_class` | **不更新** | 递增 `cnt` |
| 组内 slot 数 | 始终为 1 (`last_idx = 0`) | 根据使用量启发式算法动态确定 |

因此, `donate` 创建的组中的单个 slot 初始时处于"已释放但未激活"状态 (`freed_mask=1, avail_mask=0`)。第一次对该组进行 `malloc` 时, `try_avail` 路径会通过 `activate_group` 将 `freed_mask` 转移到 `avail_mask`, 然后正常分配。