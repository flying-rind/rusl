# fclose 函数规约

## 复杂度分级: Level 1

> musl libc 标准库文件关闭函数。刷新所有缓冲数据，关闭文件描述符，释放 `FILE` 对象内存。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 opaque 类型，由 musl 内部定义，对外表现为不透明指针
// 在 Rust 侧 extern "C" 接口中以 *mut FILE 传递
#[repr(C)]
pub struct FILE { _private: [u8; 0] }

/// 关闭文件流。刷新缓冲区、调用底层 close 回调、注销并释放 FILE 对象。
/// 永久流（stdin/stdout/stderr，带有 F_PERM 标志）不被释放。
unsafe extern "C" fn fclose(f: *mut FILE) -> c_int;
```

[Visibility]: `fclose` 声明于 `<stdio.h>`，是用户可直接调用的标准 C 库函数。在编译产物中通过 `#[no_mangle]` 以 `fclose` 符号导出，必须保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `f` 必须是通过 `fopen`、`freopen`、`fmemopen`、`open_memstream`、`fopencookie` 或 `fdopen` 成功打开的合法 `*mut FILE` 指针
- `f` 不能已被关闭（否则行为未定义）

**[Post-condition]:**

- **Case 1: 成功关闭并释放（非永久文件）**
  - `fflush(f)` 被调用以刷新所有缓冲数据
  - `f` 的底层 `close` 回调被调用以关闭文件描述符或执行自定义关闭操作
  - 若 `f` 的 `flags` 不含 `F_PERM`（即非永久流）：
    - `__unlist_locked_file(f)` 被调用（若线程锁支持已链接）
    - `f` 从全局打开文件链表（`ofl_head`）中移除
    - `f` 的 `getln_buf` 被释放
    - `f` 本身被释放（通过 `free` 或 Rust 的 `Box::from_raw` / `drop`）
  - 返回 `fflush` 与底层 `close` 的合并结果（按位 OR）

- **Case 2: 永久文件（stdin/stdout/stderr）**
  - 若 `f` 的 `flags` 包含 `F_PERM`，仅执行 `fflush` + `close` 回调
  - 不释放 FILE 对象内存，不注销链表
  - 返回 `fflush` 与 `close` 的合并结果

- **Case 3: 操作失败**
  - 返回非零值（`fflush` 或 `close` 的错误码，按位 OR）
  - 即使在关闭过程中发生错误，`FILE` 对象始终从打开文件链表中移除（永久文件除外）

**[Error Behavior]:**

| 条件 | 返回值 | 说明 |
|------|--------|------|
| `fflush` 或 `close` 成功 | `0` | 正常关闭 |
| `fflush` 失败 | 非零 | `fflush` 错误码（通常为 `EOF` / `-1`） |
| `close` 回调失败 | 非零 | `close` 回调错误码 |
| 两者都失败 | 非零 | 两者按位 OR 的结果 |

---

### 不变量

**[Invariant]:**
- 关闭后 `f` 不再有效，任何对 `f` 的访问均为未定义行为
- 即使在关闭过程中发生错误，`FILE` 对象始终从打开文件链表中移除（除非为永久文件）
- 永久流（`F_PERM` 标志）不释放内存，不注销链表

---

### 意图

关闭一个文件流。首先通过 `fflush` 刷新缓冲区，然后调用 `FILE` 对象的内部 `close` 回调（关闭底层文件描述符或调用自定义关闭函数），最后从全局打开文件链表中移除该 `FILE` 对象并释放其内存。

Rust 侧实现：
- 外部接口 `fclose` 保持 `unsafe extern "C"` 的 ABI 签名
- 内部实现可完全用安全 Rust 重写：使用 `Mutex<LinkedList>` 管理打开文件链表，使用 `Box<FILEInner>` 管理 FILE 对象生命周期
- `__unlist_locked_file(f)` 作为内部辅助函数，Rust 侧可用安全抽象替代（如从 `Vec`/`LinkedList` 中 `remove`）
- 弱符号 `__unlist_locked_file` 默认空实现：在 Rust 侧可提供一个同名的 `extern "C"` 空函数，当链接了锁支持模块时被覆盖

### 系统算法

```
fclose(f):
  1. FLOCK(f)                    // 锁定 FILE 对象
  2. r = fflush(f)               // 刷新缓冲区
  3. r |= f->close(f)            // 调用底层关闭回调
  4. FUNLOCK(f)                  // 解锁 FILE 对象
  5. if f.flags 含 F_PERM:       // 若是永久流(stdin/stdout/stderr)
        return r                 // 不释放 FILE 对象
  6. __unlist_locked_file(f)     // 从线程锁定文件链表移除
  7. __ofl_lock()                // 获取全局打开文件链表锁
  8. 从链表中移除 f:
        if f.prev: f.prev.next = f.next
        if f.next: f.next.prev = f.prev
        if *head == f: *head = f.next
  9. __ofl_unlock()              // 释放全局锁
  10. free(f.getln_buf)          // 释放 getline 缓冲区
  11. free(f)                    // 释放 FILE 对象
  12. return r
```

时间复杂度 O(1)（链表删除操作）。

---

## 依赖图

```
fclose
  ├─> fflush(f)                  (see fflush.rs spec — 刷新缓冲区)
  ├─> f->close(f)                (see __fdopen.rs spec / fmemopen.rs spec — FILE 的关闭回调)
  ├─> __unlist_locked_file(f)    (see __lockfile.rs spec — 从线程锁定文件列表中移除, weak alias)
  ├─> __ofl_lock()               (see ofl.rs spec — 获取打开文件链表锁)
  ├─> __ofl_unlock()             (see ofl.rs spec — 释放打开文件链表锁)
  ├─> free(f->getln_buf)         (来自 alloc 或自定义分配器 — 释放 getline 缓冲区)
  ├─> free(f)                    (来自 alloc 或自定义分配器 — 释放 FILE 对象)
  └─> FLOCK(f) / FUNLOCK(f)      (宏 → 内部函数, see stdio_impl.rs — 锁/解锁 FILE 对象)
```

---

## [RELY]

- `fflush`: 刷新 FILE 流缓冲区（定义于 `rusl-stdio` 内部）
- `__unlist_locked_file`: 弱符号默认空实现；链接锁支持时被覆盖为真实实现（定义于 `rusl-stdio` 内部）
- `__ofl_lock` / `__ofl_unlock`: 全局打开文件链表锁（定义于 `rusl-stdio` 的 `ofl` 模块）
- `FLOCK` / `FUNLOCK`: FILE 对象级锁（定义于 `rusl-internal` 的 `stdio_impl` 模块）
- 内存分配/释放（来自 `rusl` 内部分配器或直接使用 Rust `Box` / `Vec`）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn fclose(f: *mut FILE) -> c_int;
```

本模块保证对外提供 ABI 兼容的 `fclose` 符号。行为符合 C 标准库 `fclose()` 语义：刷新缓冲区、关闭底层文件、释放资源。对于永久流仅执行刷新和关闭回调而不释放内存。

内部符号（`dummy` / `__unlist_locked_file` 弱别名）不对外暴露，由 Rust 侧模块内部管理。
