# tre-mem 内存分配器 Rust 接口规约

## 概述

本模块为 TRE 正则表达式引擎提供 bump-pointer 内存分配器（arena allocator）。所有分配的内存块以链表形式管理，不支持单独释放，仅在 `tre_mem_destroy` 时一次性批量回收。Rust 实现中，内部可使用 `Vec<Box<[u8]>>` 或自定义链表安全抽象替代 C 的裸指针链表管理，消除手动 `malloc`/`free` 风险和内存泄漏隐患。

---

## 依赖图

```
tre_mem_new       ──→ 堆分配
tre_mem_alloc     ──→ bump-pointer 分配 + 可能的新块分配
tre_mem_calloc    ──→ tre_mem_alloc + 零初始化
tre_mem_destroy   ──→ 批量释放所有块
```

所有函数共享内部数据结构 `TreMem` / `TreBlock`。

---

## [RELY]

Predefined Structures/Functions:
  Global allocator (`std::alloc::Global` 或 libc)     // 依赖1: 底层的堆分配/释放能力
  `TRE_MEM_BLOCK_SIZE: usize = 1024`                  // 依赖2: 默认内存块大小常量
  `core::mem::align_of::<usize>()`                    // 依赖3: 指针 usize 对齐要求

---

## [GUARANTEE]

本模块所有符号均为 Internal — 不对外导出。仅 rusl crate 内部 `regcomp`、`regexec` 模块使用。

---

## 内部数据结构

### TreBlock

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct TreBlock {
    data: Box<[u8]>,       // 堆分配的数据块
}
```

**语义**: 一个内存块节点。Rust 使用 `Box<[u8]>` 替代 C 的 `void *data` 裸指针，由 Box 的 RAII 语义自动保证释放（在 `TreMem` 被 drop 时自动回收）。Rust 实现不需要单独的链表节点结构体（`tre_list_t`），可直接用 `Vec<TreBlock>` 管理所有块。

### TreMem

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct TreMem {
    blocks: Vec<TreBlock>,     // 所有已分配的内存块
    current_idx: usize,        // 当前活跃块的索引
    ptr: usize,                // 当前块中下一个可分配位置的偏移量
    n: usize,                  // 当前块中剩余可用字节数
    failed: bool,              // 分配失败标志（true = 已失败，后续分配立即返回 null）
}
```

**语义**: bump-pointer 分配器的控制结构。C 实现中的单向链表 + 独立链表节点被替换为 `Vec<TreBlock>` + 索引追踪。`current_idx` 替代 `tre_list_t *current`，`failed: bool` 替代 `int failed`。

**不变量 (Invariants)**:
- `failed == false` 时：`blocks` 非空（除非从未分配过）且 `current_idx < blocks.len()`，`ptr` 指向 `blocks[current_idx].data` 内的有效偏移，`n` 反映当前块剩余容量
- `failed == true` 时：分配器处于永久失败状态，后续所有分配请求均返回 `null()`
- `blocks` 为空 ⟹ `current_idx == 0`，`ptr == 0`，`n == 0`（初始状态）
- `current_idx` 始终有效（`< blocks.len()` 或 `blocks` 为空时 `== 0`）

---

## 内部函数

### tre_mem_new — 创建分配器

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_mem_new() -> TreMem
```

**意图**：创建并初始化一个新的 TRE 内存分配器实例。

**前置条件**：无。

**后置条件**：
- 返回初始状态的 `TreMem`：`blocks` 为空，`current_idx = 0`，`ptr = 0`，`n = 0`，`failed = false`

**Rust 设计优势**：
- C 实现中 `tre_mem_new_impl` 通过 `calloc` 堆分配 `tre_mem_struct`，需要调用者记得 `tre_mem_destroy`；Rust 实现在栈上创建 `TreMem`，通过 RAII 自动管理生命周期
- Rust 的 `Vec<TreBlock>` 自动管理块的增长，无需手动 `malloc` 链表节点
- 不需要 C 的 `provided` 参数（alloca 模式），因为 Rust 本身就支持栈分配（通过 RAII 值类型即可）

---

### tre_mem_alloc — 分配内存块

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_mem_alloc(mem: &mut TreMem, size: usize) -> *mut u8
```

**意图**：从分配器 `mem` 中分配 `size` 字节，返回对齐到 `usize` 边界的指针。

**前置条件**：
- `size > 0`
- `mem` 为有效的 `TreMem` 实例

**后置条件**：

| 条件 | 返回值 | 状态变化 |
|------|--------|----------|
| `mem.failed == true` | `null()` | 无变化（fast-fail） |
| 当前块剩余空间足够 | 对齐后的有效指针 | `ptr` 推进 `size + 对齐填充`，`n` 减少相应字节数 |
| 当前块空间不足，新块分配成功 | 对齐后的有效指针 | 新块追加到 `blocks`，`current_idx` 更新，从新块分配 |
| 新块分配失败（OOM） | `null()` | `mem.failed = true` |

**系统算法**（改编自 C 的 bump-pointer 分配）：

```
fn tre_mem_alloc(mem: &mut TreMem, size: usize) -> *mut u8 {
    if mem.failed || size == 0 {
        return null_mut();
    }

    // 计算对齐填充
    let aligned_size = align_up(size, mem::align_of::<usize>());

    // 若当前块空间不足，分配新块
    if mem.n < aligned_size {
        let block_size = max(TRE_MEM_BLOCK_SIZE, aligned_size * 8);
        let mut data = match alloc_zeroed_block(block_size) {
            Some(b) => b,
            None => { mem.failed = true; return null_mut(); }
        };
        let ptr = data.as_mut_ptr();
        mem.blocks.push(TreBlock { data });
        mem.current_idx = mem.blocks.len() - 1;
        mem.ptr = 0;
        mem.n = block_size;
    }

    // 从当前块分配
    let block = &mut mem.blocks[mem.current_idx];
    let alloc_ptr = unsafe { block.data.as_mut_ptr().add(mem.ptr) };
    mem.ptr += aligned_size;
    mem.n -= aligned_size;
    alloc_ptr
}
```

**不变量**：
- 返回的指针满足 `usize` 类型对齐要求
- 一旦 `mem.failed == true`，后续所有分配均返回 `null()`
- 分配的内存由分配器统一管理，在 `mem` 被 drop 时自动回收

---

### tre_mem_calloc — 分配并零初始化

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_mem_calloc(mem: &mut TreMem, size: usize) -> *mut u8
```

**意图**：从分配器 `mem` 中分配 `size` 字节并零初始化。

**前置条件**：
- `size > 0`
- `mem` 为有效的 `TreMem` 实例

**后置条件**：
- 与 `tre_mem_alloc` 相同，但额外保证分配的内存已零初始化
- Rust 实现中，若通过新块分配，使用 `alloc_zeroed_block` 直接获得零初始化内存；若从现有块分配，需要 `ptr::write_bytes(ptr, 0, size)` 或等价方式清零

---

### tre_mem_destroy — 销毁分配器

Rust 实现中，此函数不再需要独立存在。通过 Rust 的 RAII 机制，当 `TreMem` 值离开作用域时，`Drop` trait 自动释放 `Vec<TreBlock>` 及其包含的所有 `Box<[u8]>` 数据块。

```rust
// [Visibility]: Internal — rusl crate 内部
impl Drop for TreMem {
    fn drop(&mut self) {
        // Vec<TreBlock> 的 drop 自动递归释放所有 Box<[u8]>
        // 无需手动遍历链表和调用 free
    }
}
```

**意图**：释放 `mem` 管理的所有内存块。

**后置条件**：
- 所有 `mem.blocks` 中的数据块及其 `Vec` 存储均被释放
- `mem` 不可再被访问

**与 C 的关键差异**：
- C 中 `tre_mem_destroy` 需要遍历链表分别 `free(data)` 和 `free(list_node)`，容易出错
- Rust 中 `Drop` 由编译器自动生成或只需 `Vec<Box<[u8]>>` 的自动递归释放，安全且零泄漏
- 若某些调用场景需要提前显式释放（如在 `regcomp` 编译失败的回滚路径），可提供显式 `fn destroy(self)` 消费方法

---

## 设计意图总览

TRE 内存分配器是一种 **bump-pointer 分配器**（arena allocator），专为正则表达式编译/匹配期间大量小块分配的场景优化：

1. **批量释放**：不支持单独释放。在 Rust 中，通过 `Vec<Box<[u8]>>` + Drop 自动实现，比 C 的手动链表遍历更安全。
2. **块链扩展**：当当前块耗尽时，分配新的固定大小块（默认 1024 字节），块大小随请求大小自适应增长（`max(1024, size * 8)`）。
3. **失败传播**：`failed` 标志位确保一旦某次分配失败，后续所有分配快速失败（fail-fast）。
4. **alloca 模式废弃**：Rust 实现不再需要 C 中基于 `alloca` 的栈分配变体。通过 Rust 的 RAII 值语义，分配器可在栈上创建，`Box<[u8]>` 的数据则来自堆，退出作用域时自动回收——既比 C 的安全（无需手动 destroy），也比 alloca 的安全（无栈溢出风险，失败时安全返回 null）。

---

## 依赖关系

| 依赖 | 来源 | 可见性 |
|------|------|--------|
| `std::alloc` (Global allocator) | Rust std | Internal (通过 `Box<[u8]>` 间接使用) |
| `TRE_MEM_BLOCK_SIZE` | `tre.rs` 模块 | Internal |
| `core::mem::align_of` | Rust core | Internal |

| 被依赖 | 说明 |
|--------|------|
| `regcomp` 模块 | 管理正则编译期间的所有临时分配（AST 节点、TNFA 转移边等） |
| `regexec` 模块 | 管理正则匹配期间的运行时分配（回溯栈、标签数组等） |
