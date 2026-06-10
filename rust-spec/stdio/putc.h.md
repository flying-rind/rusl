# putc.h 内部模块规约

## 复杂度分级: Level 2

> musl libc 内部 stdio 字符写入辅助模块的 Rust 设计。定义 `do_putc` 和 `locking_putc` 两个内部函数，为 `fputc`、`putc`、`putchar` 等公开 API 提供统一的加锁字符写入逻辑。

---

## 模块接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局

/// MAYBE_WAITERS: FILE.lock 字段高位标志，表示可能有其他线程在等待该锁
const MAYBE_WAITERS: c_int = 0x40000000;

/// 完整的"加锁-写入-解锁"原子操作。
/// 使用原子 CAS 获取锁，若锁已被占用则阻塞等待，写入完成后释放锁并唤醒等待者。
pub(crate) fn locking_putc(c: c_int, f: *mut FILE) -> c_int;

/// 智能锁检查的字符写入入口函数。
/// 若 FILE 已处于免锁模式或当前线程已持有锁，则直接调用免锁写入；
/// 否则走完整的 locking_putc 加锁路径。
#[inline]
pub(crate) fn do_putc(c: c_int, f: *mut FILE) -> c_int;
```

[Visibility]: **Internal** — 本模块中所有符号均为 `pub(crate)` 或更小可见性，不对外导出，仅供 stdio 模块内部其他文件（如 `fputc`、`putc`、`putchar` 等实现）调用。`MAYBE_WAITERS` 为模块私有常量。

> 注：在 C 侧，`putc.h` 是头文件，通过 `#include` 在每个翻译单元中内联展开。在 Rust 侧，本模块可设计为独立的 `.rs` 源文件或 `mod.rs` 子模块，通过 `pub(crate) use` 或直接模块路径暴露给 stdio 的其他子模块。`MAYBE_WAITERS` 常量与 `getc.h` 中定义相同（`0x40000000`），在 Rust 侧可统一于 `stdio_impl` 模块中定义，避免重复。

---

## 前置/后置条件

### locking_putc

**[Pre-condition]:**
- `c`: 要写入的字符（以 `c_int` 传递，内部取低 8 位转为 `unsigned char`）
- `f`: 非 NULL 的 `*mut FILE` 指针，指向有效的 `FILE` 对象
- 当前线程**不持有** `(*f).lock`（锁由其他线程持有或空闲）
- `(*f).lock >= 0`（FILE 对象是带锁的常规 FILE，非免锁 FILE）

**[Post-condition]:**
- **Case 1 成功写入字符**
  - 返回写入的字符（`0`-`255` 的 `c_int` 值）
  - 锁已释放（`(*f).lock` 恢复为未持有状态，必要时唤醒等待者）

- **Case 2 写入失败**
  - 返回 `EOF`（即 `-1`）
  - 锁已释放

**[Error Behavior]:**
- 写入失败时返回 `EOF`，锁状态保证被清理
- 可能阻塞等待锁（当 `(*f).lock` 被其他线程持有时）

---

### do_putc

**[Pre-condition]:**
- `c`: 要写入的字符
- `f`: 非 NULL 的 `*mut FILE` 指针

**[Post-condition]:**
- 返回值同 `putc_unlocked`：成功返回写入的字符（`0`-`255` 的正值），失败返回 `EOF`
- 不改变 `(*f).lock` 的所有权状态（不获取也不释放锁，除非内部走 `locking_putc` 路径）

**[Error Behavior]:**
- 同 `putc_unlocked` 的错误行为

---

## 不变量

**[Invariant]:**
- `do_putc` 采用与 `do_getc` 完全对称的三路锁判断：
  1. `lock < 0` → 免锁 FILE（如 `fmemopen` 创建的流），直接免锁写入
  2. `lock != 0 && (lock & !MAYBE_WAITERS) == __pthread_self().tid` → 当前线程持有锁，直接免锁写入
  3. 其他情况 → 走完整的 `locking_putc` 加锁路径
- `locking_putc` 在任何返回路径上都保证释放锁，不会泄漏锁所有权
- `MAYBE_WAITERS` 常量值 `0x40000000` 与 musl 原定义一致，建议与 `getc.h` 共享同一常量定义
- 在 Rust 侧，`do_putc` 加 `#[inline]` 属性以保持与 C 侧 `static inline` 等效的性能特征

---

## 意图

提供 stdio 字符写入的加锁基础设施，封装三种使用场景的锁策略：

1. **调用者已持有锁**：例如 `fwrite` 内部循环——`do_putc` 检测到当前线程 `tid` 与锁持有者匹配，跳过加锁直接调用免锁写入
2. **FILE 为免锁流**：例如 `fmemopen`——`do_putc` 检测到 `lock < 0`，直接免锁写入
3. **常规路径**：需要完整加锁——`do_putc` 委托给 `locking_putc` 执行原子加锁-写入-解锁

Rust 侧实现要点：
- 在 C 侧，`putc.h` 是头文件，`do_putc` 和 `locking_putc` 作为 `static inline`/`static` 函数在每个翻译单元中复制。在 Rust 侧，这些函数定义为一个独立的内部模块，通过 `pub(crate)` 导出给 stdio 的其他子模块使用。
- `do_putc` 使用 `#[inline]` 属性，以便在调用点内联展开，保持与 C 侧 `static inline` 相同的零开销抽象
- `locking_putc` 使用 `#[inline(never)]` 属性（对应 C 侧的 `__attribute__((__noinline__))`），避免在 `do_putc` 的慢路径中过度内联导致代码膨胀
- 参数 `c` 在内部被截断为 `unsigned char`（`c as u8`），与 C 语义一致
- 原子操作（`a_cas`、`a_swap`）通过 `core::sync::atomic` 的 `AtomicI32` 实现，使用 `Ordering::Acquire`/`Ordering::Release` 语义
- `__lockfile` 阻塞等待的语义通过内部锁抽象或 futex 封装实现
- `__pthread_self()` 用于获取当前线程标识，与 `(*f).lock` 比较判断锁所有权
- 内部实现可使用 safe Rust 封装原子操作和锁逻辑，只在必要的 FFI 边界使用 `unsafe`
- 由于该模块完全内部使用，无需维持 ABI 兼容性
- `do_putc` 与 `do_getc` 结构对称，可考虑共享锁判断逻辑的公共辅助函数

## 系统算法

### locking_putc

```
locking_putc(c: c_int, f: *mut FILE) -> c_int:
  1. 使用原子 CAS 尝试获取锁:
     if a_cas(&(*f).lock, 0, MAYBE_WAITERS - 1) != 0:
       __lockfile(f)                     // 锁已被占用，阻塞等待
  2. ret = putc_unlocked(c, f)          // 执行无锁字符写入
  3. 使用原子 swap 释放锁:
     old = a_swap(&(*f).lock, 0)
     if old & MAYBE_WAITERS != 0:
       __wake(&(*f).lock, 1, 1)         // 有等待者，唤醒一个
  4. return ret
```

### do_putc

```
do_putc(c: c_int, f: *mut FILE) -> c_int:
  l = (*f).lock
  if l < 0:                             // 免锁 FILE (如 fmemopen)
    return putc_unlocked(c, f)
  if l != 0 and (l & !MAYBE_WAITERS) == __pthread_self().tid:
    return putc_unlocked(c, f)          // 当前线程已持有锁
  return locking_putc(c, f)             // 需要完整加锁
```

---

## 依赖图

```
do_putc (pub(crate), #[inline])
  ├── putc_unlocked(c_int, *mut FILE) -> c_int        (see putc_unlocked spec)
  │     └── __overflow(*mut FILE, c_int) -> c_int     (see __overflow spec)
  ├── locking_putc(c_int, *mut FILE) -> c_int         (同模块)
  │     ├── a_cas(&i32, i32, i32) -> i32              (core::sync::atomic)
  │     ├── __lockfile(*mut FILE)                     (see __lockfile spec)
  │     ├── putc_unlocked (同上)
  │     ├── a_swap(&i32, i32) -> i32                  (core::sync::atomic)
  │     └── __wake(*mut c_int, c_int, c_int)          (futex 唤醒，内部实现)
  └── __pthread_self() -> *mut pthread                (see pthread_self spec)

MAYBE_WAITERS (const, 模块私有, 建议与 getc.h 共享定义)
```

---

## [RELY]

- `putc_unlocked` / `__overflow` — 免锁底层字符写入（见 `putc_unlocked` spec / `__overflow` spec）
- `core::sync::atomic::AtomicI32` — 原子 CAS（`compare_exchange`）和原子 swap（`swap`）操作
- `__lockfile` — 阻塞获取 FILE 锁（见 `__lockfile` spec）
- `__wake` — futex 唤醒原语（内部实现）
- `__pthread_self` — 获取当前线程标识（见 `pthread_self` spec）
- `FILE` 结构体定义 — 特别是 `lock: c_int` 字段的布局（见 `stdio_impl` 模块）

## [GUARANTEE]

**无对外导出符号**。本模块为纯内部实现，不提供任何 `#[no_mangle]` 或 `extern "C"` 符号。

Internal Interface:
```
pub(crate) fn do_putc(c: c_int, f: *mut FILE) -> c_int;
pub(crate) fn locking_putc(c: c_int, f: *mut FILE) -> c_int;
const MAYBE_WAITERS: c_int = 0x40000000;
```

本模块保证为 stdio 内部提供安全的加锁字符写入抽象，封装三种锁策略（免锁流、已持有锁、需加锁），供 `fputc`、`putc`、`putchar` 等公开 API 的实现使用。内部实现细节不对外暴露，可在不破坏外部 ABI 的前提下自由重构。
