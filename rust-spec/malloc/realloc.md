# src/malloc/realloc.rs 规约 (Rust)

## 概述

`realloc.rs` 是 rusl 中 POSIX `realloc` 函数的 Rust 入口模块。该模块本身仅是一个薄封装层，实际的内存管理逻辑由 rusl 内部的新一代 malloc 实现 "mallocng" 提供。`realloc` 公共入口直接委托给 mallocng 模块中定义的内部 `realloc_impl` 函数。

本规约递归追踪 `realloc` 的全部内部依赖，按拓扑排序呈现。

---

## 依赖图

```
realloc (External, extern "C", ABI 兼容)
  └── realloc_impl (Internal, rusl::malloc::mallocng::realloc)
        ├── malloc(n) [= malloc_impl]    ── see mallocng/malloc.rs spec
        ├── free(p)   [= free_impl]      ── see mallocng/free.rs spec
        ├── core::ptr::copy_nonoverlapping       ── Rust core 库（no_std 兼容）
        ├── mremap(base, len, new_len, flags)    ── rusl 内部 syscall 封装
        ├── size_overflows(n)                ── 内部方法 (meta 模块)
        ├── get_meta(p)                      ── 内部方法 (meta 模块)
        ├── get_slot_index(p)                ── 内部方法 (meta 模块)
        ├── get_stride(g)                    ── 内部方法 (meta 模块)
        ├── get_nominal_size(p, end)         ── 内部方法 (meta 模块)
        ├── set_size(p, end, n)              ── 内部方法 (meta 模块)
        ├── size_to_class(n)                 ── 内部方法 (meta 模块)
        ├── struct Meta / struct Group        ── 内部类型 (meta 模块)
        └── 常量: UNIT, IB, MMAP_THRESHOLD  ── (meta 模块)
```

---

## 内部类型定义

### `Meta` (内部类型)

```rust
// 位于 rusl::malloc::meta 模块，repr(C) 用于与 syscall 交互时保证布局
#[repr(C)]
struct Meta {
    prev: *mut Meta,
    next: *mut Meta,
    mem: *mut Group,
    avail_mask: AtomicI32,
    freed_mask: AtomicI32,
    // 位域打包为一个 usize:
    //   [0:4]   last_idx   (5 bits)
    //   [5]     freeable   (1 bit)
    //   [6:11]  sizeclass  (6 bits)
    //   [12..]  maplen     (remaining bits)
    packed_fields: usize,
}
```

[Visibility]: Internal -- rusl mallocng 内部元数据结构，POSIX 标准未定义

**Intent**: 每个内存分配组 (group) 对应一个 `Meta` 实例，记录该组的槽位可用性、空闲状态、大小类别、mmap 长度等元数据。`Meta` 实例通过侵入式双向循环链表串联在 `Ctx.active[sc]` 队列中。

**字段说明 (通过 accessor 方法访问位域)**:

| 字段 | 含义 |
|------|------|
| `prev`, `next` | 侵入式双向循环链表指针，用于将 Meta 挂入 `active` 队列 |
| `mem` | 指向所属 `Group` 的指针 |
| `avail_mask` | 原子位掩码 (AtomicI32)，标记当前可用（已激活但未分配）的槽位 |
| `freed_mask` | 原子位掩码 (AtomicI32)，标记已被释放的槽位 |
| `last_idx()` | 该组中最后一个槽位的索引 (0-based)，组内槽位数为 `last_idx() + 1` |
| `freeable()` | 标记该组是否可以被整体回收释放 |
| `sizeclass()` | 大小类别编号 (0-47 为常规类别, 48+ 为特殊类别, 63 表示单独 mmap) |
| `maplen()` | 若通过独立 mmap 分配，记录映射的页数 (4096 字节为单位)；否则为 0 |

**位域访问器方法**:

```rust
impl Meta {
    fn last_idx(&self) -> usize { (self.packed_fields & 0x1F) as usize }
    fn set_last_idx(&mut self, v: usize) { self.packed_fields = (self.packed_fields & !0x1F) | (v as usize & 0x1F); }
    fn freeable(&self) -> bool { (self.packed_fields >> 5) & 1 != 0 }
    fn set_freeable(&mut self, v: bool) { if v { self.packed_fields |= 1 << 5; } else { self.packed_fields &= !(1 << 5); } }
    fn sizeclass(&self) -> usize { ((self.packed_fields >> 6) & 0x3F) as usize }
    fn set_sizeclass(&mut self, v: usize) { self.packed_fields = (self.packed_fields & !(0x3F << 6)) | ((v as usize & 0x3F) << 6); }
    fn maplen(&self) -> usize { self.packed_fields >> 12 }
    fn set_maplen(&mut self, v: usize) { self.packed_fields = (self.packed_fields & 0xFFF) | (v << 12); }
}
```

**设计说明**: 原 C 代码中使用位域实现紧凑存储，Rust 中改用 `usize` 整数字段 + 位操作方法，避免 Rust 位域的不稳定特性，同时保持内存布局紧凑。`avail_mask` 和 `freed_mask` 使用 `AtomicI32` 实现无锁并发访问。

---

### `Group` (内部类型)

```rust
// 位于 rusl::malloc::meta 模块
#[repr(C)]
struct Group {
    meta: *mut Meta,
    active_idx: u8, // 仅低 5 位有效 (0-31)
    // 填充至 UNIT(16) 字节对齐
    _pad: [u8; UNIT - core::mem::size_of::<*mut Meta>() - 1],
    // storage 为柔性数组，在 Rust 中通过指针运算访问
}
```

[Visibility]: Internal -- rusl mallocng 内部数据布局结构

**Intent**: 内存分配组的数据布局。实际内存以 mmap 分配，`Group` 结构体仅描述头部布局。`storage` 区域包含 `last_idx+1` 个槽位，通过指针偏移访问。组头部占 `UNIT`(16) 字节。

---

## 内部辅助函数 (meta 模块，为 realloc 直接依赖)

### `size_overflows` (内部函数)

```rust
fn size_overflows(n: usize) -> bool
```

[Visibility]: Internal -- rusl mallocng 内部辅助函数，仅 `pub(crate)` 可见

**前置条件**:
- 无特定要求

**后置条件**:
- Case 1 (溢出): 若 `n >= usize::MAX / 2 - 4096`，设置 `errno = ENOMEM`，返回 `true`
- Case 2 (正常): 否则返回 `false`，`errno` 不变

**Intent**: 在分配前检查请求大小是否会导致后续计算溢出（如加上 IB+UNIT 后溢出 usize::MAX）。

---

### `get_slot_index` (内部函数)

```rust
unsafe fn get_slot_index(p: *const u8) -> usize
```

[Visibility]: Internal -- rusl mallocng 内部辅助函数

**前置条件**:
- `p` 指向一个由 mallocng 分配的有效内存块的起始地址（用户可见指针）
- `p.add(-3)` 指向隐藏头部，其低 5 位存储了槽位索引

**后置条件**:
- 返回 `(*p.add(-3) as usize) & 31`，即该指针所在组的槽位索引 (0-31)

**Intent**: 从用户指针的隐藏头部字节中提取槽位索引。

---

### `get_meta` (内部函数)

```rust
unsafe fn get_meta(p: *const u8) -> &'static Meta
```

[Visibility]: Internal -- rusl mallocng 内部辅助函数

**前置条件**:
- `p` 指向一个由 mallocng 分配的有效内存块起始地址
- `(p as usize) % 16 == 0`（16 字节对齐）
- `p.add(-2)` 处存储了到组头部的偏移量 (以 UNIT=16 为单位)
- 若 `*p.add(-4) != 0`（表示使用了非零偏移 enframe），则 `p.add(-8)` 处存储 32 位扩展偏移量

**后置条件**:
- 通过双重间接寻址定位到 `&Meta`：
  1. 解析偏移量得到 `*const Group`
  2. 通过 `(*base).meta` 得到 `&Meta`
- 返回前执行完整的完整性断言检查（debug 模式下 panic，release 模式下由 `debug_assert!` 优化掉）：
  - `meta.mem == base`
  - `index <= meta.last_idx()`
  - 该槽位既不在 `avail_mask` 也不在 `freed_mask` 中
  - `meta_area.check == ctx.secret`（防止元数据 corruption）
  - 对于 sizeclass < 48 的组，偏移量在对应类别允许的范围内
  - 对于 mmap 组 (maplen > 0)，偏移量不超过映射范围
  - sizeclass >= 48 时断言 `meta.sizeclass() == 63`

**Intent**: 从用户指针逆向定位到管理该内存块的 `Meta`，是 mallocng 设计的核心。

---

### `get_nominal_size` (内部函数)

```rust
unsafe fn get_nominal_size(p: *const u8, end: *const u8) -> usize
```

[Visibility]: Internal -- rusl mallocng 内部辅助函数

**前置条件**:
- `p` 指向用户内存块起始地址
- `end` 指向该槽位的结束边界（已减去 IB）
- `*p.add(-3)` 的高 3 位存储了保留大小编码

**后置条件**:
- 返回 `end as usize - p as usize - reserved`，即用户数据的实际可用大小 (`old_size`)
- `reserved` 的解析：
  - `reserved = (*p.add(-3) as usize) >> 5`
  - 若 `reserved < 5`，直接使用
  - 若 `reserved == 5`，从 `end.add(-4)` 处读取 32 位扩展保留值
- 断言检查：`reserved <= end as usize - p as usize`
- 哨兵字节检查：`*end.sub(reserved) == 0`，`*end == 0`

**Intent**: 从编码的隐藏头部信息中解码出原始分配给用户的大小。

---

### `get_stride` (内部函数)

```rust
fn get_stride(g: &Meta) -> usize
```

[Visibility]: Internal -- rusl mallocng 内部辅助函数

**前置条件**:
- `g` 指向有效的 `Meta`

**后置条件**:
- Case 1 (mmap 单槽组): 若 `g.last_idx() == 0 && g.maplen() != 0`，返回 `g.maplen() * 4096 - UNIT`
- Case 2 (常规组): 否则返回 `UNIT * SIZE_CLASSES[g.sizeclass()]`

**Intent**: 返回该组中单个槽位的总跨度（stride）。

---

### `size_to_class` (内部函数)

```rust
fn size_to_class(n: usize) -> usize
```

[Visibility]: Internal -- rusl mallocng 内部辅助函数

**前置条件**:
- `n` 为用户请求的分配大小（字节）

**后置条件**:
- 返回对应的大小类别编号 (0-47)，用于索引 `SIZE_CLASSES` 和 `CTX.active`
- 算法：
  1. `n = (n + IB - 1) >> 4` -- 将字节数向上取整为 16 字节单元数
  2. 若 `n < 10`，直接返回 `n`
  3. 否则 `n += 1`，使用 `n.leading_zeros()` 计算前导零数量，结合固定查找表 `SIZE_CLASSES` 确定类别

**Intent**: 将用户请求大小映射到 mallocng 的 48 个大小类别之一。

---

### `set_size` (内部函数)

```rust
unsafe fn set_size(p: *mut u8, end: *mut u8, n: usize)
```

[Visibility]: Internal -- rusl mallocng 内部辅助函数

**前置条件**:
- `p` 指向用户内存块起始地址
- `end` 指向槽位边界（`p.add(stride - IB)`）
- `n` 为新的用户可用大小，满足 `n <= end as usize - p as usize`

**后置条件**:
- 将新的大小 `n` 编码到隐藏头部：
  - `reserved = end as usize - p as usize - n`
  - 若 `reserved > 0`，在 `*end.sub(reserved) = 0`
  - 若 `reserved >= 5`，在 `*end.sub(4)` 写入 32 位扩展保留值，在 `*end.sub(5) = 0`
  - 将 `*p.sub(3)` 更新为 `(*p.sub(3) & 31) | ((reserved as u8) << 5)`

**Intent**: 将新分配大小编码到内存块的隐藏头部。

---

## 内部实现函数

### `realloc_impl` (内部函数)

```rust
// 位于 rusl::malloc::mallocng::realloc 模块
pub(crate) unsafe fn realloc_impl(p: *mut core::ffi::c_void, n: usize) -> *mut core::ffi::c_void
```

[Visibility]: Internal -- rusl 内部实现函数。不对外导出，仅在 `pub(crate)` 范围内可见。用户程序应通过外部包装的公开 `realloc` 函数调用。

**Intent**: rusl mallocng 的 realloc 核心实现。采用多级策略，按优先级递减尝试：原地大小调整（最优，零拷贝） -> mremap 重映射（mmap 大块场景） -> malloc+copy_nonoverlapping+free（通用回退路径）。

---

#### 前置条件

- 若 `p` 不为 null，`p` 必须是先前由 `malloc()`、`calloc()`、`realloc()` 或兼容分配函数返回的有效指针，且尚未被 `free()` 或 `realloc()` 释放
- 若 `p` 为 null，行为等同于 `malloc_impl(n)`
- 无特定锁持有要求（内部通过 `malloc`/`free` 自行管理锁）

#### 后置条件

**Case 1: `p` 为 null (等效于 malloc)**

- 直接调用 `malloc_impl(n)` 分配新内存
- 返回分配得到的指针，若分配失败则返回 `null`

**Case 2: `n` 导致溢出 (`size_overflows(n)` 为 `true`)**

- 返回 `null`，设置 `errno = ENOMEM`
- 原内存块 `p` 保持有效且未被释放

**Case 3: 原地缩容/扩容 (最优路径，零拷贝)**

- **触发条件** (三个条件同时满足):
  1. `n <= avail_size` -- 新大小不超过槽位可用空间
  2. `n < MMAP_THRESHOLD` (131052 字节)
  3. `size_to_class(n) + 1 >= g.sizeclass()` -- 新大小类别与原类别兼容

- **计算过程**:
  - `g = get_meta(p)` -- 定位元数据
  - `idx = get_slot_index(p)` -- 获取槽位索引
  - `stride = get_stride(g)` -- 获取槽位跨度
  - `start = (*g.mem).storage_ptr().add(stride * idx)` -- 槽位起始地址
  - `end = start.add(stride - IB)` -- 槽位可用末尾
  - `avail_size = end as usize - p as usize` -- 可用字节数

- **动作**: 调用 `set_size(p, end, n)` 就地更新记录的大小
- **返回**: 原指针 `p`（内存地址不变）

**Case 4: mremap 重映射 (mmap 大块优化路径)**

- **触发条件** (两个条件同时满足):
  1. `g.sizeclass() >= 48` -- 原块为大对象（独立 mmap 分配）
  2. `n >= MMAP_THRESHOLD` (131052 字节)

- **前置断言**: `g.sizeclass() == 63`

- **计算过程**:
  - `base = p as usize - start as usize` -- 用户数据在 mmap 区域内的偏移量
  - `needed = ((n + base + UNIT + IB + 4095) & !4095)` -- 页对齐的新映射大小

- **子情况 4a: 新大小恰好等于原大小**
  - `g.maplen() * 4096 == needed` -- 无需重新映射
  - 直接复用 `new = g.mem`

- **子情况 4b: 需要 mremap**
  - 调用 `sys_mremap(g.mem, g.maplen() * 4096, needed, MREMAP_MAYMOVE)`
  - 此时 rusl 内部通过 `asm!` 内联汇编直接发起 `SYS_mremap` 系统调用

- **成功处理**:
  - 若 `new != MAP_FAILED`:
    - 更新元数据：`g.mem = new`，`g.set_maplen(needed / 4096)`
    - 重新计算用户指针和边界
    - 写入尾部哨兵：`*end = 0`
    - 调用 `set_size(p, end, n)` 更新大小记录
    - 返回更新后的 `p`
  - 若 `new == MAP_FAILED`，继续执行 Case 5

**Case 5: malloc+copy_nonoverlapping+free (通用回退路径)**

- **动作**:
  1. `new = malloc_impl(n)` -- 分配新内存块
  2. 若 `new` 为 null，返回 `null`（`errno = ENOMEM`），原块 `p` 保持有效
  3. 调用 `core::ptr::copy_nonoverlapping(p as *const u8, new as *mut u8, min(n, old_size))`
  4. `free_impl(p)` -- 释放旧内存块
  5. 返回 `new`

**Rust 设计要点**:
- 用 `core::ptr::copy_nonoverlapping` 替代 C 的 `memcpy`，两者在 `#![no_std]` 环境下均可使用，语义等价
- `mremap` 系统调用通过 `asm!` 内联汇编直接发起，不依赖 `libc` crate，符合 rusl 设计原则
- 内部使用 `unsafe` 块管理裸指针操作，但在 unsafe 块内部加 `debug_assert!` 进行安全性检查
- `avail_mask` / `freed_mask` 的原子操作使用 `AtomicI32::fetch_or` / `AtomicI32::compare_exchange` 代替 C 的 `a_or` / `a_cas`

---

#### 不变量

- **Inv 1 (数据安全)**: 在 Case 2 失败和 Case 5 malloc 失败时，原内存块 `p` 始终保持有效且内容不变
- **Inv 2 (原地调整安全性)**: Case 3 的原地缩容/扩容保证了 `n <= end - p`
- **Inv 3 (sizeclass 单调性)**: Case 3 中的条件 `size_to_class(n) + 1 >= g.sizeclass()` 确保新大小类别不低于原类别太多
- **Inv 4 (哨兵字节)**: 每次成功调用 `set_size` 后，`*end` 处有哨兵字节，用于 free 时的完整性验证

---

## 对外导出函数

### `realloc` (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn realloc(p: *mut core::ffi::c_void, n: usize) -> *mut core::ffi::c_void
```

[Visibility]: External -- POSIX.1-2001 / ISO C89 标准函数，`<stdlib.h>` 声明。通过 `extern "C"` 和 `#[no_mangle]` 保证 ABI 兼容性，外部 C 代码可透明调用。

**ABI 兼容性保证**:
- 使用 `extern "C"` 调用约定
- 参数 `p: *mut c_void` 对应 C 的 `void *p`
- 参数 `n: usize` 对应 C 的 `size_t n`
- 返回值 `*mut c_void` 对应 C 的 `void *`
- 使用 `#[no_mangle]` 确保符号名为 `realloc`

**意图**: 更改 `p` 指向的内存块大小为 `n` 字节。实现委托给内部 `realloc_impl`。

**前置条件**:
- 若 `p` 不为 null，`p` 必须是先前由 `malloc()`、`calloc()`、`realloc()`、`aligned_alloc()` 或 `posix_memalign()` 返回的有效指针，且尚未被 `free()` 或 `realloc()` 释放
- 若 `p` 为 null，函数等价于 `malloc(n)`
- 若 `n == 0` 且 `p` 不为 null，musl 的行为等价于 `free(p)` 并返回 `null`

**后置条件**:
- **成功**: 返回指向新分配内存块的指针
  - 若原地调整或 mremap 成功，返回的指针可能等于原 `p`
  - 若需要移动（Case 5），返回新指针，旧块已被释放
  - 若 `n > 旧大小`，超出部分的**内容未初始化**
- **失败**: 返回 `null`，`errno = ENOMEM`，原内存块 `p` 保持不变

**实现架构**:
```
rusl/src/malloc/realloc.rs           -- 公共入口 (extern "C" fn realloc)
rusl/src/malloc/mallocng/realloc.rs  -- 核心实现逻辑 (realloc_impl)
rusl/src/malloc/mallocng/meta.rs     -- 元数据结构定义与辅助方法
```

**线程安全性**: 通过内部 `malloc_impl`/`free_impl` 的锁机制保证线程安全。

---

## 常量定义

| 常量 | 定义位置 | 值 | 含义 |
|------|---------|-----|------|
| `UNIT` | meta.rs | 16 | 基本分配单元大小（字节），所有对齐的基础 |
| `IB` | meta.rs | 4 | 槽位末尾保留的 in-band 元数据字节数 |
| `MMAP_THRESHOLD` | meta.rs | 131052 | 超过此大小的分配使用独立 mmap 而非槽位分配 |
| `MREMAP_MAYMOVE` | syscall.rs | 1 | mremap 标志，允许内核移动映射 |
| `MAP_FAILED` | syscall.rs | `-1_isize as *mut c_void` | mremap 失败时的返回值 |
| `SIZE_CLASSES` | meta.rs (声明), mallocng/malloc.rs (定义) | [u16; 48] | 每个大小类别对应的最大分配单元数 |

---

## 跨文件依赖说明

| 依赖项 | 来源 | 类型 | 说明 |
|--------|------|------|------|
| `malloc_impl` | `rusl::malloc::mallocng::malloc` | 内部实现 | Case 1 和 Case 5 中分配新内存 |
| `free_impl` | `rusl::malloc::mallocng::free` | 内部实现 | Case 5 中释放旧内存 |
| `core::ptr::copy_nonoverlapping` | Rust `core` crate | 标准 core 库（no_std 兼容） | Case 5 中拷贝数据，替代 C 的 `memcpy` |
| `sys_mremap` | `rusl::syscall` | 内部 syscall 封装 | Case 4 中通过 `asm!` 发起 SYS_mremap |
| `Meta` / `Group` | `rusl::malloc::mallocng::meta` | 内部类型 | 元数据结构和分配组头部 |
| `get_meta` / `get_slot_index` / `get_stride` / `get_nominal_size` / `set_size` / `size_to_class` / `size_overflows` | `rusl::malloc::mallocng::meta` | 内部辅助函数 | 已在本文档详述 |
| `SIZE_CLASSES` | `rusl::malloc::mallocng::meta` (声明, `pub(crate)`), `rusl::malloc::mallocng::malloc` (定义) | 内部全局常量 | 48 个元素的大小类别表 |
| `CTX` | `rusl::malloc::mallocng::malloc` | 内部全局变量 | 全局分配器上下文 |

## 依赖符号归类

### 对外导出符号 (External)

| 符号 | Rust 签名 | 说明 |
|------|-----------|------|
| `realloc` | `pub unsafe extern "C" fn realloc(p: *mut c_void, n: usize) -> *mut c_void` | POSIX 标准函数，必须保持 ABI 兼容 |

### 内部依赖符号 (Internal -- 不对外导出)

| 符号 | 来源 | 可见性 |
|------|------|--------|
| `realloc_impl` | `rusl::malloc::mallocng::realloc` | `pub(crate)` |
| `malloc_impl` | `rusl::malloc::mallocng::malloc` | `pub(crate)` |
| `free_impl` | `rusl::malloc::mallocng::free` | `pub(crate)` |
| `Meta` / `Group` | `rusl::malloc::mallocng::meta` | `pub(crate)` |
| `get_meta` / `get_slot_index` / `get_stride` 等辅助函数 | `rusl::malloc::mallocng::meta` | `pub(crate)` |
| `SIZE_CLASSES` | `rusl::malloc::mallocng::meta` / `malloc` | `pub(crate)` |
| `CTX` | `rusl::malloc::mallocng::malloc` | `pub(crate)` |
| `sys_mremap` | `rusl::syscall` | `pub(crate)` |
| `UNIT` / `IB` / `MMAP_THRESHOLD` | `rusl::malloc::mallocng::meta` | `pub(crate)` |

## 模块划分建议

遵循 C 源码的架构，rusl 中对应的模块划分为：

```
rusl/src/malloc/
  realloc.rs              -- 公共入口 (extern "C" fn realloc), 委托给 realloc_impl
  mallocng/
    mod.rs                -- 模块入口, 重新导出 pub(crate) 项
    realloc.rs            -- realloc_impl 核心实现
    malloc.rs             -- malloc_impl 核心实现
    free.rs               -- free_impl 核心实现
    meta.rs               -- Meta/Group 结构体定义及内联辅助函数
    syscall.rs            -- mmap/mremap/munmap/brk/madvise 系统调用封装 (通过 asm!)
    glue.rs               -- 锁原语、随机密钥生成、命名空间映射
```

[RELY]
Predefined Structures/Functions:
  // 来自 meta 模块 (rusl::malloc::mallocng::meta)
  struct Meta { ... };                      // 依赖1: 核心元数据结构
  struct Group { ... };                     // 依赖2: 分配组数据结构
  fn size_overflows(n: usize) -> bool;      // 依赖3: 溢出检查
  fn size_to_class(n: usize) -> usize;      // 依赖4: 大小类别映射
  unsafe fn get_meta(p: *const u8) -> &Meta;       // 依赖5: 指针反查元数据
  unsafe fn get_slot_index(p: *const u8) -> usize; // 依赖6: 提取槽位索引
  fn get_stride(g: &Meta) -> usize;         // 依赖7: 获取槽位跨度
  unsafe fn get_nominal_size(p: *const u8, end: *const u8) -> usize; // 依赖8: 解码原始大小
  unsafe fn set_size(p: *mut u8, end: *mut u8, n: usize); // 依赖9: 编码新大小
  const UNIT: usize = 16;                   // 依赖10: 基本分配单元
  const IB: usize = 4;                      // 依赖11: in-band 元数据大小
  const MMAP_THRESHOLD: usize = 131052;     // 依赖12: mmap 阈值
  const SIZE_CLASSES: [u16; 48];            // 依赖13: 大小类别表

  // 来自 malloc 模块 (rusl::malloc::mallocng::malloc)
  unsafe fn malloc_impl(n: usize) -> *mut c_void;  // 依赖14: 核心分配函数

  // 来自 free 模块 (rusl::malloc::mallocng::free)
  unsafe fn free_impl(p: *mut c_void);             // 依赖15: 核心释放函数

  // 来自 syscall 模块 (rusl::syscall)
  unsafe fn sys_mremap(old: *mut c_void, old_len: usize, new_len: usize, flags: c_int) -> *mut c_void;
                                                   // 依赖16: mremap 系统调用封装

  // 标准库依赖 (no_std 环境可用)
  core::ptr::copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize);
                                                   // 依赖17: 内存拷贝 (替代 memcpy)

[GUARANTEE]
Exported Interface:
  // 本模块保证对外提供的接口签名，必须满足 ABI 兼容性
  #[no_mangle]
  pub unsafe extern "C" fn realloc(p: *mut core::ffi::c_void, n: usize) -> *mut core::ffi::c_void;
  // POSIX.1-2001 / C89 标准函数
  // - p = NULL 时等价于 malloc(n)
  // - 成功时返回新指针（可能等于 p）
  // - 失败时返回 NULL，errno = ENOMEM，原块 p 保持有效
  // - n = 0 且 p != NULL 时等价于 free(p) 并返回 NULL