# __stdout_write 函数规约

## 复杂度分级: Level 2

> musl libc 内部 stdout 专用写函数实现。在首次写入 stdout 时，将 `f->write` 替换为 `__stdio_write`，并探测终端窗口大小以决定是否启用行缓冲模式。

---

## 函数接口

```rust
use core::ffi::c_int;

extern "C" fn __stdout_write(f: *mut FILE, buf: *const u8, len: usize) -> usize;
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。在 `__fdopen` 初始化时被设置为 stdout 的 `f->write` 函数指针。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: `*mut FILE`，stdout 文件流，非空指针
- `buf` / `len`: 同 `__stdio_write` 的参数要求
- 若 `(*f).flags` 已设置 `F_SVB`，则跳过终端探测

**[Post-condition]:**

**Case 1: 首次调用（初始化逻辑执行）**
- `(*f).write` 被替换为 `__stdio_write`（此后不再进入此函数，后续调用直接走 `__stdio_write`）
- 若 stdout 是终端且无 `F_SVB`：`(*f).lbf = '\n'`（行缓冲模式保持）
- 若 stdout 不是终端且无 `F_SVB`：`(*f).lbf = -1`（关闭行缓冲，退化为全缓冲/无缓冲）
- 若 `F_SVB` 已设置：`(*f).lbf` 不变
- 返回值同 `__stdio_write(f, buf, len)`

**Case 2: 非首次调用（`f->write` 已被替换）**
- 不再进入此函数，直接走 `__stdio_write` 路径

---

### 不变量

**[Invariant]:**
- 此函数指针在第一次调用后被替换为 `__stdio_write`，因此是幂等的（仅首次执行初始化逻辑）
- `F_SVB` 标志（stdio variable buffer）保护 `(*f).lbf` 不被 ioctl 探测覆盖——若用户通过 `setvbuf` 设置了缓冲模式，则不应被终端检测覆盖
- 替换 `f->write` 指针的操作必须在实际写入之前执行（防止并发竞态，不过在单线程初始化阶段无此问题）

---

### 意图

stdout 的延迟初始化写函数。首次调用时：
1. 将 `f->write` 替换为 `__stdio_write`（此后不再执行初始化逻辑）
2. 通过 `ioctl(TIOCGWINSZ)` 检测 stdout 是否为终端以决定行缓冲模式
3. 转发实际写入到 `__stdio_write`

Rust 侧实现：
- 首次调用检测：通过将 `f->write` 自身替换为 `__stdio_write` 来实现"仅执行一次"语义
- `ioctl` 系统调用通过 `syscall!` 宏实现
- `winsize` 结构体定义为 `#[repr(C)]` 与 C 侧兼容
- 函数指针替换：`(*f).write = __stdio_write as ...`（需通过函数指针类型的类型转换）
- 内部可封装终端检测为独立函数：`fn is_terminal(fd: c_int) -> bool`
- 注意：`f->lbf = -1` 在 Rust 中对应 `-1_i32 as c_int`，表示关闭行缓冲

---

### 系统算法

```
__stdout_write(f, buf, len):
  /* 1. 覆盖 write 函数指针，延迟初始化只执行一次 */
  (*f).write = __stdio_write

  /* 2. 检测终端并配置行缓冲 */
  if ((*f).flags & F_SVB) == 0:
    wsz: struct winsize (未初始化)
    if syscall!(SYS_ioctl, (*f).fd, TIOCGWINSZ, &wsz) != 0:  // ioctl 失败
      (*f).lbf = -1                        // 非终端，关闭行缓冲
    // ioctl 成功：保持 (*f).lbf = '\n'（行缓冲模式）

  /* 3. 执行实际写入 */
  return __stdio_write(f, buf, len)
```

时间复杂度 O(1)（不含 `__stdio_write` / ioctl 系统调用开销）。

---

## 依赖图

```
__stdout_write
  ├─> __stdio_write       (see __stdio_write spec)
  ├─> syscall!(SYS_ioctl)  (内核)
  └─> struct winsize      (repr(C) 平台相关结构体)
```

---

## [RELY]

- `__stdio_write` — 默认写操作（本模块）
- `syscall!` 宏 — 系统调用接口（`SYS_ioctl`, `TIOCGWINSZ`）
- `Winsize` — 终端窗口大小结构体（`#[repr(C)]`，与 C 的 `struct winsize` 兼容）
- 常量: `F_SVB`, `TIOCGWINSZ`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __stdout_write(f: *mut FILE, buf: *const u8, len: usize) -> usize;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为与原 C 实现完全一致：首次调用时完成 stdout 延迟初始化（终端检测 + `write` 函数指针替换），随后转发所有写操作到 `__stdio_write`。
