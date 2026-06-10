# fflush 函数规约

## 复杂度分级: Level 1

> musl libc 标准库流刷新函数。将流缓冲区的未写入数据写出到实际文件/设备，同步读取位置。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 opaque 类型（定义同 fclose.rs spec）
#[repr(C)]
pub struct FILE { _private: [u8; 0] }

/// 刷新 FILE 流的缓冲区。
/// - 若 f 非 NULL：刷新该特定流的缓冲区
/// - 若 f 为 NULL：刷新所有当前打开的流（包括 stdout/stderr，若已链接）
unsafe extern "C" fn fflush(f: *mut FILE) -> c_int;

// weak_alias: fflush_unlocked 是 fflush 的弱别名，共享同一实现
// 行为与 fflush 完全相同，但不执行内部 FILE 对象锁定（调用者需自行保证线程安全）
unsafe extern "C" fn fflush_unlocked(f: *mut FILE) -> c_int;
```

[Visibility]: `fflush` 和 `fflush_unlocked` 声明于 `<stdio.h>`，是用户可直接调用的 POSIX 标准接口。两者在编译产物中均以 `#[no_mangle]` 导出，必须保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- 若 `f != NULL`:
  - `f` 必须是一个有效的已打开 `*mut FILE` 指针
  - 调用者（对于 `fflush`，非 `fflush_unlocked`）无需持有锁，函数内部会锁住 `f`
- 若 `f == NULL`:
  - 触发全局刷新所有打开文件流（包括 `stdout`/`stderr` 若已链接）

**[Post-condition]:**

- **Case 1: f != NULL 且成功**
  - 若 `f` 的写入缓冲区有待写数据（`wpos != wbase`）：
    - 调用底层 `write` 回调将缓冲区数据写出
    - 若写出后 `wpos == 0`（write 返回错误），返回 `EOF`
  - 若 `f` 的读取缓冲区有未消费的预读数据（`rpos != rend`）：
    - 调用底层 `seek` 回调将文件偏移量回退到实际已读位置
  - 清除所有读写模式指针：`wpos = wbase = wend = 0; rpos = rend = 0`
  - 返回 `0`

- **Case 2: f == NULL（全局刷新）**
  - 若 `__stdout_used != NULL`，刷新 stdout
  - 若 `__stderr_used != NULL`，刷新 stderr
  - 遍历所有打开文件链表，对每个有未写数据的流调用 `fflush`
  - 返回所有 `fflush` 调用的合并结果（按位 OR）

- **Case 3: f != NULL 且写出失败**
  - 底层 `write` 将 `wpos` 设为 `0` 以标记写入错误
  - 返回 `EOF`

**[Error Behavior]:**
- 单个流刷新失败时返回 `EOF`（`-1`）
- 全局刷新时返回所有子调用的按位 OR 结果

---

### 不变量

**[Invariant]:**
- 刷新操作后，FILE 对象的读/写缓冲区被重置为空闲状态（`rpos = rend = 0`, `wpos = wbase = wend = 0`）
- 全局刷新时对打开文件链表持有锁，保证快照一致性
- `fflush_unlocked` 与 `fflush` 行为完全一致，仅跳过内部锁获取/释放

---

### 意图

刷新 `FILE` 流的缓冲区：若写入模式下有未写出数据，将其写出到实际文件/设备；若读取模式下有预读数据，将其位置同步回底层文件偏移量。若参数 `f` 为 `NULL`，则刷新所有当前打开的流。

Rust 侧实现：
- 外部接口 `fflush` 和 `fflush_unlocked` 保持 `unsafe extern "C"` 的 ABI 签名
- `fflush_unlocked` 在 Rust 侧可以通过调用内部公共辅助函数实现（该辅助函数不获取锁，`fflush` 获取锁后调用同一辅助函数）
- 全局刷新可复用 Rust 的迭代器模式遍历打开文件链表
- 内部弱符号 `__stdout_used` / `__stderr_used` 在 Rust 侧可用 `Option<*mut FILE>` 或 `AtomicPtr<FILE>` 全局变量替代

### 系统算法

```
fflush(f):
  if f == NULL:                             // 刷新所有流
    r = 0
    if __stdout_used.is_some(): r |= fflush(__stdout_used)
    if __stderr_used.is_some(): r |= fflush(__stderr_used)
    for each f in __ofl_iter():             // 遍历所有打开文件
      FLOCK(f)
      if f.wpos != f.wbase: r |= fflush(f)  // 递归调用非 NULL 分支
      FUNLOCK(f)
    __ofl_unlock()
    return r

  // 刷新单个流
  FLOCK(f)
  if f.wpos != f.wbase:                     // 写入模式: 有未写出数据
    f.write(f, 0, 0)                        // 调用底层写出
    if f.wpos == 0:                         // write 将 wpos 置 0 表示错误
      FUNLOCK(f); return EOF
  if f.rpos != f.rend:                      // 读取模式: 有未消费的预读数据
    f.seek(f, f.rpos - f.rend, SEEK_CUR)    // 回退文件位置

  f.wpos = f.wbase = f.wend = 0             // 重置写入缓冲区
  f.rpos = f.rend = 0                       // 重置读取缓冲区
  FUNLOCK(f)
  return 0
```

时间复杂度：单流刷新 O(1)，全局刷新 O(n)（n = 打开文件数）。

---

## 依赖图

```
fflush
  ├─> __stdout_used / __stderr_used (weak refs, see stdout.rs / stderr.rs)
  ├─> __ofl_lock() / __ofl_unlock() (see ofl.rs spec)
  ├─> FLOCK(f) / FUNLOCK(f)         (宏 → 内部函数, see stdio_impl.rs)
  ├─> f->write(f, 0, 0)             (通过 FILE 对象函数指针调用)
  └─> f->seek(f, ...)               (通过 FILE 对象函数指针调用)
fflush_unlocked  (weak_alias of fflush)
```

---

## [RELY]

- `__stdout_used` / `__stderr_used`: 弱符号，指向 stdout/stderr 的全局指针（定义于 `rusl-stdio` 的 `stdout` / `stderr` 模块）
- `__ofl_lock` / `__ofl_unlock`: 全局打开文件链表锁（定义于 `rusl-stdio` 的 `ofl` 模块）
- `FLOCK` / `FUNLOCK`: FILE 对象级锁宏（定义于 `rusl-internal` 的 `stdio_impl` 模块）
- `write` / `seek` 函数指针: 由创建流的模块设置（`__fdopen`、`fmemopen`、`fopencookie`、`open_memstream` 等）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn fflush(f: *mut FILE) -> c_int;
unsafe extern "C" fn fflush_unlocked(f: *mut FILE) -> c_int;
```

本模块保证对外提供 ABI 兼容的 `fflush` 和 `fflush_unlocked` 符号。`fflush_unlocked` 与 `fflush` 行为一致，区别仅在于不执行 FILE 对象级锁定（调用者自行保证线程安全）。行为符合 POSIX `fflush()` 语义：将流缓冲区数据写出，重置缓冲区状态；`NULL` 参数时刷新所有打开流。

内部弱符号 `__stdout_used` / `__stderr_used`（及 `dummy` 占位符）不对外暴露，由 Rust 侧模块内部管理。
