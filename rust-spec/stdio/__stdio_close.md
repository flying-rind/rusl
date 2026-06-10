# __stdio_close 函数规约

## 复杂度分级: Level 1

> musl libc 内部 FILE 默认关闭操作实现。作为 `f->close` 函数指针的默认值，通过 `close` 系统调用关闭文件描述符，并在关闭前调用 AIO 清理回调。

---

## 函数接口

```rust
use core::ffi::c_int;

extern "C" fn __stdio_close(f: *mut FILE) -> c_int;
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。作为 `f->close` 函数指针的默认值，被 `fclose`、`__fclose_ca` 等间接调用。

---

### 内部依赖符号

C spec 中的 `static int dummy(int fd)`（AIO 弱别名的默认实现）在 Rust 侧可以：
- **省略不生成**：AIO 子系统若未被链接，可直接将 `__aio_close` 的默认行为内联为一个返回 `fd` 的恒等函数
- 或重新设计为内部普通函数：`fn aio_close_default(fd: c_int) -> c_int { fd }`，使用 `#[no_mangle]` 仅当需要作为弱别名链接点时保留

**设计决策**：由于 Stage 0 不需要 AIO 支持，`__aio_close` 可直接实现为恒等函数（或通过 weak 链接在 linker 层面处理）。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: `*mut FILE`，非空指针，其 `fd` 为有效的文件描述符

**[Post-condition]:**

**Case 1: 关闭成功**
- `(*f).fd` 引用的文件描述符被关闭（通过 `syscall!(SYS_close)`）
- `__aio_close(f->fd)` 已完成清理（若 AIO 子系统已链接则为实际清理，否则为恒等无操作）
- 返回 `0`（`close` 系统调用成功返回值）

**Case 2: 关闭失败**
- 返回 `-1`，errno 由 `close` 系统调用设置

**[Error Behavior]:**
- 本函数不自行设置 errno，错误由底层 `close` 系统调用设置（如 `EBADF`）

---

### 不变量

**[Invariant]:**
- `__aio_close` 始终先于 `close` 系统调用执行
- `f` 的其他字段在关闭操作后可能变为无效（由调用方负责后续处理）

---

### 意图

关闭 `FILE` 关联的文件描述符。在关闭前调用 `__aio_close(f->fd)` 以允许 AIO 子系统执行必要的清理。

Rust 侧实现：
- `__aio_close`：Stage 0 直接实现为恒等函数 `fn __aio_close(fd: c_int) -> c_int { fd }`，后续阶段可替换为真正的 AIO 清理
- `close` 系统调用通过 `syscall!` 宏实现
- 函数签名保持 `extern "C"` 以兼容 `f->close` 函数指针类型
- 内部可用 `core::ptr::NonNull<FILE>` 表达非空语义（仅在内部，`extern "C"` 边界仍使用 `*mut FILE`）

---

### 系统算法

```
__stdio_close(f):
  /* 1. 运行 AIO 清理（若 AIO 子系统已链接，则为实际清理；否则为无操作） */
  aio_fd = __aio_close((*f).fd)

  /* 2. 关闭文件描述符 */
  return syscall!(SYS_close, aio_fd)
```

时间复杂度 O(1)。

---

## 依赖图

```
__stdio_close
  └─> syscall!(SYS_close)   (内核)
```

Stage 0 不依赖 AIO 子系统。未来 Stage 可能增加 `__aio_close` 依赖。

---

## [RELY]

- `syscall!` 宏 — 系统调用接口（`SYS_close`）
- (可选，未来 Stage) `__aio_close` — AIO 关闭回调

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __stdio_close(f: *mut FILE) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为与原 C 实现完全一致：先执行 AIO 清理（Stage 0 为无操作），再关闭文件描述符，返回 `close` 系统调用的结果。
