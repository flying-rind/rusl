# free.rs 规约 (rusl mallocng 实现)

> **实现架构说明**：`src/malloc/free.rs` 仅定义一层薄封装，将 POSIX `free(void *p)` 转发给内部实现 `__libc_free`。实际分配器算法位于 `src/malloc/mallocng/` 模块，与 musl C 实现对应。本规约涵盖完整依赖链，内部依赖符号均按 Rust 安全设计哲学重新设计。

---

## 依赖图

```
free (POSIX, src/malloc/free.rs)  [extern "C" ABI]
  └── __libc_free (即 mallocng free, src/malloc/mallocng/free.rs)  [Internal, crate-private]
        ├── get_meta(p)                  [meta.rs, pub(crate) fn] — 从指针恢复 &Meta
        ├── get_slot_index(p)            [meta.rs, pub(crate) fn] — 从指针提取槽位索引
        ├── get_stride(g)                [meta.rs, pub(crate) fn] — 获取组的步长
        ├── get_nominal_size(p, end)     [meta.rs, pub(crate) fn] — 验证并返回分配大小
        ├── nontrivial_free(g, idx)      [free.rs, fn] — 慢路径释放
        │     ├── okay_to_free(g)        [free.rs, fn] — 判断是否应释放整组
        │     ├── free_group(g)          [free.rs, fn] — 释放整个组
        │     │     ├── step_seq()            [meta.rs, pub(crate) fn]
        │     │     ├── record_seq(sc)        [meta.rs, pub(crate) fn]
        │     │     ├── get_meta(p)           [meta.rs, pub(crate) fn]
        │     │     ├── get_slot_index(p)     [meta.rs, pub(crate) fn]
        │     │     ├── nontrivial_free()     [递归]
        │     │     └── free_meta(g)          [meta.rs, pub(crate) fn]
        │     ├── queue() / dequeue()         [meta.rs, pub(crate) fn] — 活动链表操作
        │     └── freed_mask.fetch_or()       [core::sync::atomic::AtomicI32] — 原子位设置
        ├── wrlock() / unlock()           [glue.rs, pub(crate) fn] — 锁操作
        ├── sys_madvise(MADV_FREE)        [syscall.rs, pub(crate) fn] — 页面回收提示
        └── sys_munmap()                  [syscall.rs, pub(crate) fn] — 解除内存映射
```

---

## 完整符号依赖分析（递归追踪）

### 第一层：对外导出符号（External）

| 符号 | C 签名 | Rust 签名 | 来源模块 | 说明 |
|------|--------|-----------|----------|------|
| `free` | `void free(void *p)` | `pub unsafe extern "C" fn free(p: *mut c_void)` | `src/malloc/free.rs` | POSIX/C 标准函数，必须保持 C ABI 兼容 |

### 第二层：直接内部依赖（Internal，__libc_free 实现层）

| 符号 | C 类型 | Rust 类型 | 来源模块 | Visibility | 说明 |
|------|--------|-----------|----------|------------|------|
| `get_meta` | `static inline` | `pub(crate) fn get_meta(p: *const u8) -> &Meta` | `meta.rs` | Internal | 指针反查 Meta，含多重断言 |
| `get_slot_index` | `static inline` | `pub(crate) fn get_slot_index(p: *const u8) -> usize` | `meta.rs` | Internal | 提取 slot 索引 |
| `get_stride` | `static inline` | `pub(crate) fn get_stride(g: &Meta) -> usize` | `meta.rs` | Internal | 计算槽位步长 |
| `get_nominal_size` | `static inline` | `pub(crate) fn get_nominal_size(p: *const u8, end: *const u8) -> usize` | `meta.rs` | Internal | 恢复原始分配大小 |
| `wrlock` | `static inline` | `pub(crate) fn wrlock()` | `glue.rs` | Internal | 获取写锁 |
| `unlock` | `static inline` | `pub(crate) fn unlock()` | `glue.rs` | Internal | 释放锁 |
| `a_cas` | `atomic.h 宏` | `core::sync::atomic::AtomicI32::compare_exchange` | Rust core | 标准库 | CAS 原子操作 |
| `a_or` | `atomic.h 宏` | `core::sync::atomic::AtomicI32::fetch_or` | Rust core | 标准库 | 原子按位或 |
| `sys_madvise` | Linux syscall | `pub(crate) fn sys_madvise(addr: *mut c_void, len: usize, advice: c_int) -> c_int` | `syscall.rs` | Internal | madvise 系统调用 |
| `sys_munmap` | Linux syscall | `pub(crate) fn sys_munmap(addr: *mut c_void, len: usize) -> c_int` | `syscall.rs` | Internal | munmap 系统调用 |

### 第三层：nontrivial_free 间接依赖（Internal）

| 符号 | C 类型 | Rust 类型 | 来源模块 | Visibility | 说明 |
|------|--------|-----------|----------|------------|------|
| `okay_to_free` | `static int` | `fn okay_to_free(g: &Meta) -> bool` | `free.rs` | `pub(crate)` | bounce 启发式判断 |
| `free_group` | `static struct mapinfo` | `fn free_group(g: &Meta) -> MapInfo` | `free.rs` | `pub(crate)` | 释放整组 |
| `queue` | `static inline` | `pub(crate) fn queue(head: &mut Option<NonNull<Meta>>, m: &mut Meta)` | `meta.rs` | Internal | 循环链表插入 |
| `dequeue` | `static inline` | `pub(crate) fn dequeue(head: &mut Option<NonNull<Meta>>, m: &mut Meta)` | `meta.rs` | Internal | 循环链表移除 |
| `activate_group` | `static inline` | `pub(crate) fn activate_group(m: &Meta) -> u32` | `meta.rs` | Internal | 激活组 |
| `free_meta` | `static inline` | `pub(crate) fn free_meta(m: &mut Meta)` | `meta.rs` | Internal | 回收 Meta 到空闲链表 |

### 第四层：okay_to_free 依赖（Internal）

| 符号 | C 类型 | Rust 类型 | 来源模块 | Visibility | 说明 |
|------|--------|-----------|----------|------------|------|
| `get_stride` | `static inline` | (见第二层) | `meta.rs` | Internal | 已列出 |
| `is_bouncing` | `static inline` | `pub(crate) fn is_bouncing(sc: usize) -> bool` | `meta.rs` | Internal | 查询 bounce 状态 |
| `size_classes` | `extern const uint16_t[]` | `pub(crate) static SIZE_CLASSES: [u16; 48]` | `malloc.rs` | Internal | 大小类别表 |
| `ctx.usage_by_class` | `size_t[48]` | `ctx.usage_by_class: [usize; 48]` | `malloc.rs` | Internal | 全局使用量统计 |

### 第五层：free_group 依赖（Internal）

| 符号 | C 类型 | Rust 类型 | 来源模块 | Visibility | 说明 |
|------|--------|-----------|----------|------------|------|
| `step_seq` | `static inline` | `pub(crate) fn step_seq()` | `meta.rs` | Internal | 推进全局序列号 |
| `record_seq` | `static inline` | `pub(crate) fn record_seq(sc: usize)` | `meta.rs` | Internal | 记录 unmap 序列号 |
| `get_meta` | `static inline` | (见第二层) | `meta.rs` | Internal | 已列出 |
| `get_slot_index` | `static inline` | (见第二层) | `meta.rs` | Internal | 已列出 |
| `nontrivial_free` | `static` | (递归调用) | `free.rs` | Internal | 递归 |
| `free_meta` | `static inline` | (见第三层) | `meta.rs` | Internal | 已列出 |

### 第六层：全局数据结构依赖

| 符号 | C 定义 | Rust 定义 | 来源模块 | Visibility |
|------|--------|-----------|----------|------------|
| `Meta` | `struct meta` | `pub(crate) struct Meta { ... }` | `meta.rs` | Internal |
| `Group` | `struct group` | `pub(crate) struct Group { ... }` | `meta.rs` | Internal |
| `MetaArea` | `struct meta_area` | `pub(crate) struct MetaArea { ... }` | `meta.rs` | Internal |
| `MallocContext` | `struct malloc_context` | `pub(crate) struct MallocContext { ... }` | `meta.rs` | Internal |
| `MapInfo` | `struct mapinfo` | `pub(crate) struct MapInfo { ... }` | `free.rs` | Internal |
| `ctx` | `struct malloc_context` | `pub(crate) static CTX: MallocContext` | `malloc.rs` | Internal |
| `size_classes` | `const uint16_t[]` | `pub(crate) static SIZE_CLASSES: [u16; 48]` | `malloc.rs` | Internal |
| `__malloc_lock` | `int[1]` | `pub(crate) static MALLOC_LOCK: Mutex<()>` | `lock.rs` | Internal |
| `__syscall(SYS_munmap)` | C 宏 | `unsafe fn sys_munmap(addr: *mut c_void, len: usize) -> c_int` | `syscall.rs` | Internal |
| `__syscall(SYS_madvise)` | C 宏 | `unsafe fn sys_madvise(addr: *mut c_void, len: usize, advice: c_int) -> c_int` | `syscall.rs` | Internal |

---

## [RELY]

Predefined Structures:
  // --- 来自 meta.rs 的核心数据结构 ---
  pub(crate) struct Meta { ... };
                                  // 依赖 1: 分配组元数据，含 prev/next/mem/avail_mask/freed_mask/
                                  //        last_idx/freeable/sizeclass/maplen 位域字段
  pub(crate) struct Group { ... };
                                  // 依赖 2: 分配组结构，含 meta 反向指针/active_idx/storage[]
  pub(crate) struct MetaArea { ... };
                                  // 依赖 3: Meta 页对齐容器，含 check 校验值/next/nslots/slots[]
  pub(crate) struct MallocContext { ... };
                                  // 依赖 4: 全局分配器上下文，含 secret/pagesize/init_done/
                                  //        mmap_counter/free_meta_head/avail_meta*/meta_area_*/
                                  //        active[48]/usage_by_class[48]/unmap_seq[32]/
                                  //        bounces[32]/seq/brk 等字段
  pub(crate) struct MapInfo { base: *mut c_void, len: usize };
                                  // 依赖 5: munmap 信息传递结构，{0, 0} 表示无需 unmap

Predefined Functions (来自 meta.rs):
  pub(crate) fn get_meta(p: *const u8) -> &Meta;
                                  // 依赖 6: 从用户指针反查 &Meta，含 14 层断言校验
  pub(crate) fn get_slot_index(p: *const u8) -> usize;
                                  // 依赖 7: 提取 p[-3] & 31 作为槽位索引
  pub(crate) fn get_stride(g: &Meta) -> usize;
                                  // 依赖 8: 返回组内每个槽位的字节步长
  pub(crate) fn get_nominal_size(p: *const u8, end: *const u8) -> usize;
                                  // 依赖 9: 从 reserved 字段恢复原始分配大小并校验溢出守卫
  pub(crate) fn free_meta(m: &mut Meta);
                                  // 依赖 10: 清零 Meta 并归还到 ctx.free_meta_head 空闲链表
  pub(crate) fn queue(head: &mut Option<core::ptr::NonNull<Meta>>, m: &mut Meta);
                                  // 依赖 11: 将 Meta 插入双向循环链表尾部
  pub(crate) fn dequeue(head: &mut Option<core::ptr::NonNull<Meta>>, m: &mut Meta);
                                  // 依赖 12: 从双向循环链表中移除 Meta
  pub(crate) fn activate_group(m: &Meta) -> u32;
                                  // 依赖 13: 原子地将 freed_mask 中的槽位移到 avail_mask
  pub(crate) fn step_seq();
                                  // 依赖 14: 递增 ctx.seq（0..255 循环），溢出时重置所有 unmap_seq
  pub(crate) fn record_seq(sc: usize);
                                  // 依赖 15: 记录某 size class 最近一次 unmap 的序列号
  pub(crate) fn is_bouncing(sc: usize) -> bool;
                                  // 依赖 16: 查询某 size class 是否处于抖动状态

Predefined Functions (来自 glue.rs):
  pub(crate) fn wrlock();
                                  // 依赖 17: 获取全局 malloc 写锁 (多线程模式下)
  pub(crate) fn unlock();
                                  // 依赖 18: 释放全局 malloc 锁

Predefined Functions (来自 syscall.rs):
  pub(crate) unsafe fn sys_munmap(addr: *mut c_void, len: usize) -> c_int;
                                  // 依赖 19: Linux munmap 系统调用，通过 asm!("syscall") 发起
  pub(crate) unsafe fn sys_madvise(addr: *mut c_void, len: usize, advice: c_int) -> c_int;
                                  // 依赖 20: Linux madvise 系统调用，通过 asm!("syscall") 发起

Predefined Globals (来自 malloc.rs):
  pub(crate) static CTX: MallocContext;
                                  // 依赖 21: 全局唯一分配器上下文实例
  pub(crate) static SIZE_CLASSES: [u16; 48];
                                  // 依赖 22: 大小类别查找表 (以 UNIT 为单位)

Predefined Globals (来自 glue.rs / lock.rs):
  pub(crate) fn is_multi_threaded() -> bool;
                                  // 依赖 23: 运行时多线程检测 (对应 C 的 MT 宏)
  pub(crate) const USE_MADV_FREE: bool = false;
                                  // 依赖 24: 编译期 MADV_FREE 开关 (默认为 false)

[GUARANTEE]
Exported Interface:
  pub unsafe extern "C" fn free(p: *mut c_void);
                                  // 本模块保证对外提供的 C ABI 兼容接口签名
                                  // 语义: 释放先前由 malloc/calloc/realloc/aligned_alloc
                                  //       返回的内存块；p == NULL 时无操作

Crate-Private Interface (供 rusl 内部模块调用):
  pub(crate) unsafe fn __libc_free(p: *mut c_void);
                                  // 内部符号: 供 realloc(ptr, 0)、atexit 清理、
                                  // stdio 缓冲区释放等 rusl 内部组件使用
                                  // 与 C 的 __libc_free 保持相同的调用约定，
                                  // 但其实现内部使用 Rust 安全抽象

---

## Level 1: 简单规约 (Public API)

### free (对外导出, src/malloc/free.rs)

```rust
pub unsafe extern "C" fn free(p: *mut c_void);
```

**[Visibility]: External -- C 标准函数，`<stdlib.h>` 声明，POSIX 标准要求。必须保持 C ABI 兼容。**

- **前置条件 (Precondition)**:
  - `p` 必须是之前由 `malloc`、`calloc`、`realloc`、`aligned_alloc` 或 `posix_memalign` 返回的有效指针，**或**为 `NULL`
  - 若 `p` 非空，其指向的内存必须尚未被释放（double-free 会导致未定义行为，rusl 通过断言和头部失效化提供 best-effort 检测）
  - 调用者不持有任何 `malloc` 相关的内部锁（本函数内部自行处理同步）

- **后置条件 (Postcondition)**:
  - **Case 1: `p.is_null()`**: 函数立即返回，无任何操作。这是 C 标准要求的无操作行为。
  - **Case 2: `p` 非空**: 指针指向的内存被标记为可供后续分配重用。释放后 `p` 自身的值不变，但变为悬垂指针，再次解引用或释放均为未定义行为。

- **错误处理**: 无返回值，不设置 `errno`（C 标准规定 `free()` 不报告错误）

- **线程安全**: 完全线程安全。通过内部锁保护全局分配器状态，并在 fast-path 路径上使用 `core::sync::atomic::AtomicI32::compare_exchange` 无锁 CAS 原子操作优化高并发场景。

- **信号安全**: 不是 async-signal-safe。持有锁期间被信号中断可能导致死锁。

- **Intent**: 将一块动态分配的内存归还给分配器，使其可被后续 `malloc` 调用重用。实现采用分层策略：fast-path 原子释放（无锁）处理同组内非首/非末释放；slow-path 加锁处理边界情况（首/末释放触发整组回收、mmap 解除映射等）。

---

## Level 3: 高度优化设计 (内部实现, 系统算法)

### __libc_free (即 mallocng free, 内部符号)

```rust
pub(crate) unsafe fn __libc_free(p: *mut c_void);
```

**[Visibility]: Internal -- rusl crate 私有符号。供 `realloc(ptr, 0)`、`atexit` 清理、`stdio` 缓冲区释放等内部模块直接调用。不对外部用户暴露。**

- **前置条件**: 同 public `free`，但此外还要求 rusl 的 malloc 子系统已完成初始化（首次调用 malloc/calloc/realloc 时会自动初始化，详见 `malloc.rs` spec）

- **后置条件**: 同 public `free`

- **意图 (Intent)**: rusl mallocng 分配器的核心释放逻辑。采用三级处理路径：
  1. **Fast-path（无锁原子释放）**: 若释放的槽位不是组内首/末活跃槽位，直接原子 CAS 更新 `freed_mask`，零锁竞争
  2. **Slow-path（加锁释放）**: 若释放触发组边界条件（最后一个被使用槽位或首个释放），加锁调用 `nontrivial_free` 执行复杂的组管理逻辑
  3. **Page-level reclamation**: 在释放前，对大槽位中完整的空闲物理页通过 `sys_madvise(MADV_FREE)` 向内核提示可回收

- **系统算法 (System Algorithm)**:

  **第一阶段: 元数据定位与验证**

  从用户指针 `p` 恢复分配器内部元数据。rusl mallocng 继承 musl 的独特设计，使用指针前 4 字节（`IB = 4`）作为 out-of-band header：
  ```
  [group header (UNIT bytes)]
  [slot 0: ...]
  [slot 1: ...]
  ...
  对于每个槽位中分配的块:
    p-4: 溢出标志字节 (0 或非零)
    p-3: bit[4:0] = slot index; bit[7:5] = reserved size
    p-2,p-1: uint16_t offset from group base (以 UNIT=16 为单位)
    p:    用户数据起始
  ```

  算法步骤：
  1. `g = get_meta(p)`: 从 `p[-2]` 读取到组基址的偏移量（2 字节或 4 字节大偏移），定位 `&Group`，再通过 `group.meta` 获取 `&Meta`。同时验证 `meta_area.check == ctx.secret`（防 corruption）。Rust 版本以 `Option<&Meta>` 或带 `debug_assert!` 的方式实现校验。
  2. `idx = get_slot_index(p)`: 提取 `p[-3] & 31` 作为槽位索引
  3. `stride = get_stride(g)`: 对于 mmap 单槽组，stride = maplen*4096 - UNIT；否则 stride = UNIT*SIZE_CLASSES[sc]
  4. 计算 `start = g.mem.storage.as_ptr().add(stride*idx)` 和 `end = start.add(stride - IB)`
  5. `get_nominal_size(p, end)`: 验证槽位的 reserved 字段和溢出字节，确认数据完整性

  **第二阶段: 写防护字节（double-free 检测辅助）**

  ```
  *(p as *mut u8).sub(3) = 255;              // 标记为无效
  *((p as *mut u16).sub(1)) = 0;             // 清零组内偏移
  ```

  **第三阶段: 页面回收提示（MADV_FREE / lazy freeing）**

  ```
  条件: ((start.sub(1) as usize) ^ (end as usize)) >= 2*PGSZ && g.last_idx > 0
  ```
  即：当槽位跨越至少 2 个物理页边界，且组有多个槽位（非单槽 mmap 组）时：
  1. 计算槽位内完整的物理页范围（对齐到 PGSZ）
  2. 若 `USE_MADV_FREE` 为真（当前为 false，默认禁用），调用 `sys_madvise(base, len, MADV_FREE)` 告知内核可惰性回收这些页面
  3. 保存并恢复 `errno`（`sys_madvise` 可能修改 errno，C 标准要求 `free()` 不改变 errno）

  **第四阶段: Fast-path 原子释放（无锁）**

  使用 `AtomicI32` 的 CAS 进入原子无锁循环：

  ```
  // Rust 风格: 使用 AtomicI32::compare_exchange 替代 C 的 a_cas 宏
  let self_bit = 1u32 << idx;
  let all = (2u32 << g.last_idx) - 1;

  loop {
      let freed = g.freed_mask.load(Ordering::Relaxed);
      let avail = g.avail_mask.load(Ordering::Relaxed);
      let mask = freed | avail;
      debug_assert!(mask & self_bit == 0);  // double-free 检测

      // 首个释放 或 最后一个被使用槽位 → 进入 slow path
      if freed == 0 || mask + self_bit == all {
          break;
      }

      // 单线程模式: 直接赋值
      if !is_multi_threaded() {
          g.freed_mask.store(freed + self_bit, Ordering::Relaxed);
          return;
      }

      // 多线程模式: CAS 无锁更新
      match g.freed_mask.compare_exchange_weak(
          freed, freed + self_bit,
          Ordering::AcqRel, Ordering::Relaxed
      ) {
          Ok(_) => return,     // CAS 成功，释放完成
          Err(_) => continue,  // CAS 失败，重试
      }
  }
  ```

  关键洞察：fast-path 的数学条件与 C 版本相同：
  - 如果 `freed_mask == 0`（组内此前无释放），这是"首释"→ 必须 slow-path 判断是否需要 activate group
  - 如果 `mask + self_bit == all`（释放此槽位后所有槽位均 freed/avail，即成"末释"）→ 必须 slow-path 判断是否需要 free_group

  **第五阶段: Slow-path 加锁释放**

  ```
  wrlock();
  let mi = nontrivial_free(g, idx);
  unlock();
  if mi.len > 0 {
      let e = errno::get();          // Rusl: 保存 errno
      sys_munmap(mi.base, mi.len);
      errno::set(e);                  // Rusl: 恢复 errno
  }
  ```

  锁操作：`wrlock()` 在 `is_multi_threaded()` 为真时获取全局锁；非多线程模式下为空操作。

  若 `nontrivial_free` 返回需要 `munmap` 的映射范围，执行 `sys_munmap` 并保护 `errno`。

---

### nontrivial_free (内部模块函数)

```rust
pub(crate) fn nontrivial_free(g: &Meta, i: usize) -> MapInfo;
```

**[Visibility]: Internal -- `pub(crate)` 函数，仅在本 crate 内可见。rusl mallocng 内部释放逻辑的核心分支。**

- **前置条件**:
  - 必须在持有全局 malloc 写锁的情况下调用
  - `g` 指向有效的 `Meta`，其 `mem` 指向的 `Group` 存在
  - `i` 是待释放槽位在组内的索引，满足 `0 <= i <= g.last_idx`
  - 槽位 `i` 当前未被标记为 freed 或 avail（`!(g.freed_mask & (1<<i)) && !(g.avail_mask & (1<<i))`）

- **后置条件**:
  - **Case 1: 整组释放**: 当 `mask + self_bit == all`（释放此槽位后组内无活跃槽位）且 `okay_to_free(g)` 返回真时：
    - 对于多槽组（sc < 48）：从 `ctx.active[sc]` 链表中 dequeue，若被移除的是当前 active 组，激活链表中的下一个组
    - 调用 `free_group(g)` 回收整组资源
    - 返回需要 `munmap` 的 `MapInfo`（若组是 mmap'd）或 `MapInfo { base: ptr::null_mut(), len: 0 }` 零值
  - **Case 2: 标记为 avail 并重新入队**: 当此前组内无任何 freed/avail 槽位（`mask == 0`），且组不在 active 链表上时（`ctx.active[sc] != g`）：
    - 将组 `queue` 到 `ctx.active[sc]` 链表，标记为可复用
    - 设置 `g.freed_mask.fetch_or(self_bit, Ordering::Release)`
  - **Case 3: 仅标记释放**: 其他情况下，仅通过 `AtomicI32::fetch_or` 原子设置释放位

- **Intent**: 决定释放后的组管理策略。核心状态机与 C 版本一致：
  - 若释放导致组"全空"且策略允许，则彻底回收整组
  - 若组"首次出现空闲槽位"且当前未被 active 追踪，将其加入活动链表供后续分配
  - 否则仅标记该槽位为 freed，等待同组其他槽位释放

- **系统算法**: 使用位掩码管理槽位状态：
  ```
  self_bit = 1u32 << i                          // 本槽位的位掩码
  mask = g.freed_mask | g.avail_mask             // 已有空闲/可用槽位
  all  = (2u32 << g.last_idx) - 1                // 所有槽位的掩码

  条件 A: mask + self_bit == all → 释放后组全空
  条件 B: mask == 0              → 组内此前无空闲槽位
  ```

---

### free_group (内部模块函数)

```rust
pub(crate) fn free_group(g: &Meta) -> MapInfo;
```

**[Visibility]: Internal -- `pub(crate)` 函数，仅在本 crate 内可见。负责释放整个 slot group 的所有资源。**

- **前置条件**:
  - 必须在持有全局 malloc 写锁的情况下调用
  - `g` 指向有效的 `Meta`
  - 组内所有槽位已确认无活跃分配（调用者已做此判断）
  - 若 `g.next` 和 `g.prev` 非空，说明该组在某个链表上，调用者必须已将其 dequeue

- **后置条件**:
  - `Meta` 通过 `free_meta(g)` 归还给 `ctx.free_meta_head` 空闲链表
  - 若 `g.sc < 48`：更新 `ctx.usage_by_class[sc]` 减去该组贡献的槽位数
  - **Case 1: mmap 组 (`g.maplen > 0`)**:
    - 调用 `step_seq()` / `record_seq(sc)` 记录 unmap 序列号（用于 bounce 检测）
    - 返回 `MapInfo { base: g.mem.as_ptr() as *mut c_void, len: g.maplen * 4096 }` → 调用者执行 `sys_munmap`
  - **Case 2: 子分配组 (`g.maplen == 0`)**:
    - 该组是作为"大槽位内的子组"分配的
    - 将 `g.mem.meta` 置空（标记该 group header 不再有效）。Rust 使用 `NonNull<Meta>` + `Option` 或直接用裸指针表示。
    - **递归调用** `nontrivial_free(m, idx)` 释放父组中对应的槽位
    - 返回父组释放产生的 `MapInfo`（由递归调用返回）

- **Intent**: 将一组槽位所占用的全部资源归还系统或父分配器。关键设计决策：
  - mmap 组直接 `sys_munmap` 归还内核，减少进程 RSS
  - 子分配组递归归还给父组槽位，父组槽位重新变为可用

- **Rust 设计要点**: 递归调用 `nontrivial_free` 通过 `&Meta` 引用传递，Rust 借用检查器天然保证递归过程中不会产生 aliasing 问题；注意需要确保递归深度可控（组嵌套通常最多 2 层）。

---

### okay_to_free (内部模块函数)

```rust
pub(crate) fn okay_to_free(g: &Meta) -> bool;
```

**[Visibility]: Internal -- `pub(crate)` 函数，仅在本 crate 内可见。实现"bounce prevention"启发式策略，防止在特定大小类上出现分配/释放抖动。**

- **前置条件**:
  - 必须在持有全局 malloc 写锁的情况下调用
  - `g` 指向有效的 `Meta`，且组内所有槽位均已释放或即将变为可用

- **后置条件**:
  - 返回 `true` 或 `false`，不修改任何全局状态
  - 返回值指示是否应该释放该组（`true` = 释放，`false` = 保留）

- **Intent**: 在线分配器中的关键启发式 -- 阻止"bouncing"（抖动），即某个大小类频繁分配后又立即释放整组，导致反复 mmap/munmap。策略通过 `ctx.bounces[]` 数组追踪各大小类近期 unmap 频率，对抖动类保守地保留至少一个组。

- **系统算法 (7 层决策级联)**:

  ```
  (1) if !g.freeable → return false;
      → 显式标记不可释放的组（如 donate 产生的组）

  (2) if sc >= 48 || get_stride(g) < UNIT*SIZE_CLASSES[sc] → return true;
      → 大对象（>= MMAP_THRESHOLD）或步长不匹配的组总是释放

  (3) if g.maplen == 0 → return true;
      → 子分配组（嵌入在父组槽位中的组）总是释放
      → 原因：重建成本低，且可能阻塞父组的大槽位回收

  (4) if g.next != ptr::from_ref(g) → return true;
      → 若有另一个非满组，释放此组以减少碎片、合并分配

  (5) if !is_bouncing(sc) → return true;
      → 非抖动大小类，直接释放

  (6) if 9*cnt <= usage && cnt < 20 → return true;
      → 即便在抖动类中，若使用量高而组槽位数少，释放低容量组以推动创建更优组
      → cnt = g.last_idx + 1（组槽位数），usage = CTX.usage_by_class[sc]

  (7) return false;
      → 保底：抖动类中保留最后一个组，防止反复 mmap/munmap
  ```

  **Bounce 检测机制**（`is_bouncing` / `record_seq` / `decay_bounces`）:
  - 全局序列号 `ctx.seq` 递增（0..255 循环），每次 size class 7..38 的 munmap 发生时记录 `ctx.unmap_seq[sc-7] = seq`
  - `account_bounce(sc)`: 若距上次 unmap < 10 个序列号窗口，递增 `ctx.bounces[sc-7]`（上限 150）
  - `decay_bounces(sc)`: 每次成功在该类分配时递减 bounce 计数
  - `is_bouncing(sc)`: `bounces[sc-7] >= 100` 表示该类正在抖动
  - 此机制类似 TCP 拥塞控制的 AIMD 思想，用序列号窗口替代时间窗口，避免 syscall 开销

- **Rust 设计要点**: `okay_to_free` 是纯函数（除读取全局状态外），不修改任何共享状态。Rust 中可用 `&self` 参数明确表达不修改语义。读取 `CTX` 全局时由于调用者已持锁，可直接读取无需原子操作。

---

## 不变量 (Invariants)

### I1: errno 保持不变量
`free()` 的执行（包括内部 `sys_madvise` 和 `sys_munmap` 系统调用）**必须保证调用者的 `errno` 值不被改变**。任何可能修改 `errno` 的 syscall 前后必须保存/恢复 `errno`。Rust 中使用 `errno::get()` / `errno::set()` 辅助函数实现。

### I2: 元数据完整性不变量
在任何线程释放操作前后，以下性质始终成立：
- 若 `p` 是有效的已分配指针，则 `get_meta(p)` 能成功定位到正确的 `&Meta`，且 `meta_area.check == ctx.secret`
- 一个槽位不能同时出现在 `freed_mask` 和 `avail_mask` 中
- `ctx.active[sc]` 链表上的每个组，其 `avail_mask` 必须非零（即组内有可用槽位）

### I3: 锁层级不变量
- `nontrivial_free`、`free_group`、`okay_to_free` 必须在持有全局 malloc 写锁时调用
- fast-path 原子 CAS 路径不持有锁，通过 `compare_exchange` 保证 `freed_mask` 更新的原子性
- Rust 锁设计：使用 `pub(crate) fn wrlock()` / `pub(crate) fn unlock()` 封装，内部可使用 `core::sync::atomic::AtomicBool` 实现自旋锁或 futex-based 锁

### I4: usage_by_class 一致性不变量
`CTX.usage_by_class[sc]` 应等于所有 sc 类的活动组中 `last_idx + 1` 的和。当组被 `free_group` 释放时，其贡献从计数中扣减。

### I5: 单槽 mmap 组不变量
当 `g.last_idx == 0 && g.maplen > 0`（单槽 mmap 组）时，释放该槽位必定触发整组 `sys_munmap`，因此不会走 `sys_madvise(MADV_FREE)` 页面回收路径。

---

## 相关数据结构

### MapInfo (内部类型, 定义于 free.rs)

```rust
pub(crate) struct MapInfo {
    pub(crate) base: *mut core::ffi::c_void,
    pub(crate) len: usize,
}
```

**[Visibility]: Internal -- 仅在 `mallocng/free.rs` 内定义，用于在 `nontrivial_free` 和调用者之间传递需要 unmap 的内存范围信息。`MapInfo { base: ptr::null_mut(), len: 0 }` 表示无需 unmap。**

- **Rust 设计**: 替代 C 的 `struct mapinfo`。在 Rust 中，可以使用 `MapInfo::none()` 构造函数明确表达无需 unmap 语义，也可考虑用 `Option<MapInfo>` 表达（选项取决于内部代码风格）。本设计选择带哨兵值的方案以保持与 C 版本相近的控制流。

---

## 调用者（外部模块依赖）

| 调用者 | 说明 |
|--------|------|
| `realloc(ptr, 0)` | 当 realloc 的 size 为 0 时等价于 `free(ptr)`，最终调用 `__libc_free` |
| `__libc_free` 的直接调用者 | 其他 rusl crate 内部模块需直接释放内存时使用（如 atexit 清理、stdio 缓冲区释放等） |

## 被调用者（free 依赖的外部模块）

| 被调用者 | 来源 | 说明 |
|----------|------|------|
| `sys_madvise(base, len, MADV_FREE)` | `syscall.rs` | 惰性页面回收提示（当前 `USE_MADV_FREE=false`，编译期禁用） |
| `sys_munmap(base, len)` | `syscall.rs` | 释放 mmap 分配的内存映射，通过 `asm!("syscall")` 发起 |
| `AtomicI32::compare_exchange` | `core::sync::atomic` | CAS 原子操作，替代 C 的 `a_cas` |
| `AtomicI32::fetch_or` | `core::sync::atomic` | 原子按位或，替代 C 的 `a_or` |
| `wrlock()` / `unlock()` | `glue.rs` | 全局 malloc 锁封装 |

---

## Rust 设计与 C 实现的关键差异

1. **原子操作**: 使用 `core::sync::atomic::AtomicI32` 替代 C 的 `atomic.h` 宏（`a_cas`、`a_or`）。Rust 标准库提供的原子类型具有更好的类型安全和跨平台可移植性。

2. **系统调用**: 使用 `syscall.rs` 模块通过 `core::arch::asm!("syscall", ...)` 直接发起系统调用，替代 C 的 `__syscall` 宏。禁止使用 `libc` crate。

3. **指针类型**: C 的 `void *` 映射为 Rust 的 `*mut core::ffi::c_void` 或 `*mut u8`；`unsigned char *` 映射为 `*mut u8` 或 `*const u8`。

4. **断言机制**: C 使用 `assert()` 宏（release 下可能被 NDEBUG 移除），Rust 使用 `debug_assert!()`（debug 构建下检查，release 下省略）+ 关键路径使用不可优化的 `assert!()` 或手动 panic。

5. **链表操作**: C 使用裸指针操作的双向循环链表，Rust 内部实现可使用 `core::ptr::NonNull<Meta>` 表示更安全的非空指针，配合 `Option<NonNull<Meta>>` 表示可为空的链表头。

6. **锁操作**: C 的 `LOCK`/`UNLOCK` 宏通过 `lock.h` 实现自旋锁（futex-based），Rust 可实现为基于 `AtomicBool` 的自旋锁或直接使用 futex 的轻量级互斥锁（`core::sync::atomic::fence` + futex）。

7. **MapInfo 零值语义**: C 使用 `{NULL, 0}` 哨兵，Rust 可使用 `MapInfo::none()` 关联函数或 `Option<MapInfo>` 表达，提升类型安全性。

8. **全局状态**: C 的全局 `ctx` 和 `__malloc_lock` 通过 `extern` 声明跨文件共享，Rust 中可通过 `pub(crate) static` 或模块级全局变量（配合 `UnsafeCell`/`Mutex`）实现。

---

## 安全考虑

1. **Double-free 检测**: `get_meta` 中的 `debug_assert!(!(meta.avail_mask & (1u32 << index)))` 和 `debug_assert!(!(meta.freed_mask & (1u32 << index)))` 基于掩码检测 double-free。fast-path 中的 `debug_assert!(mask & self_bit == 0)` 提供早期检测。此外，free 后将 `p[-3]=255` 和 `*(p as *mut u16).sub(1) = 0` 使二次释放时 `get_meta` 必然失败。

2. **元数据 corruption 检测**: `meta_area.check == ctx.secret` 验证 meta area 完整性。`get_meta` 中的多重断言（offset 范围、size class 一致性、maplen 边界）提供深度防御。Rust 中这些校验在安全抽象内部完成，对外暴露安全的接口。

3. **errno 保持**: 所有可能修改 `errno` 的系统调用（`sys_madvise`、`sys_munmap`）前后均有保存/恢复操作，满足 C 标准要求。Rust 中使用 `errno::get()` / `errno::set()` 辅助函数。

4. **`#![no_std]` 约束**: 整个 rusl crate 为 `#![no_std]` 实现，仅依赖 `core` 和 `alloc`（若需动态内存）；所有系统调用通过 `asm!` 自实现，不依赖 `libc` crate 或任何外部 FFI 封装。

5. **Unsafe 边界**: 公共 API `free(p: *mut c_void)` 是 `unsafe extern "C"` 函数，因为调用者必须保证 `p` 是有效指针。内部实现函数在合适的 unsafe 边界内操作，利用 Rust 的类型系统在 safe 代码中操作内部数据结构。