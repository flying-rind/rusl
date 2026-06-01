// mallocng — rusl 的新一代内存分配器 (malloc new generation)
//
// 对应 musl 的 src/malloc/mallocng/ 目录。
// 所有符号为 crate-private，不对外部用户暴露。
//
// 子模块说明:
// - meta.rs:  元数据结构定义与操作函数（Meta、Group、MetaArea、MallocContext 等）
// - context.rs: 全局上下文与跨文件共享符号（SIZE_CLASSES、CTX、alloc_meta、is_allzero）
// - free.rs:  释放逻辑（__libc_free、nontrivial_free、free_group、okay_to_free）
// - glue.rs:  锁操作与多线程检测（wrlock、unlock、is_multi_threaded）
// - malloc.rs: malloc 核心分配逻辑（__libc_malloc_impl）
// - realloc.rs: 重分配逻辑（__libc_realloc）
// - syscall.rs: 系统调用封装（mmap、munmap、mremap、madvise 等）
// - dynlink.rs: 动态链接替换标志
// - aligned_alloc.rs: 对齐分配逻辑（aligned_alloc）
// - donate.rs: 内存捐献逻辑（__malloc_donate、donate）
// - malloc_usable_size.rs: 查询已分配内存块的实际可用大小（malloc_usable_size）
//
// 注意: context.rs 是 meta.rs 的依赖，定义了跨文件共享的全局符号。
// 若与其他模块（如 malloc.rs）存在符号重复定义，需在后续整合中统一。

#![allow(dead_code, unused_imports)]

pub(crate) mod meta;
pub(crate) mod context;
pub(crate) mod free;
pub(crate) mod glue;
pub(crate) mod malloc;
pub(crate) mod realloc;
pub(crate) mod syscall;
pub(crate) mod dynlink;
pub(crate) mod aligned_alloc;
pub(crate) mod donate;
pub(crate) mod malloc_usable_size;