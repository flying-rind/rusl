# stderr 规约

## 复杂度分级: Level 1

> musl libc 标准错误输出流全局变量的 Rust 实现。包含 `stderr`（对外导出）、`__stderr_FILE`（内部实现）和 `__stderr_used`（内部哨兵变量）。

---

## 全局接口

```rust
// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 struct _IO_FILE
// 此处以不透明指针形式呈现，保证 ABI 兼容性

// stderr: 指向标准错误输出 FILE 对象的常量指针
// 对应 C 的 FILE *const stderr = &__stderr_FILE;
#[no_mangle]
pub static stderr: *mut FILE;
```

[Visibility]:
- `stderr` — **User**，标准 C 库全局变量（ISO C），声明于 `<stdio.h>`，用户程序通过 `stderr` 宏直接使用
- `__stderr_FILE` — **Internal**，`#[no_mangle]` 导出以保持符号可见性，但不对用户暴露；Rust 侧使用 `pub(crate)` 可见性
- `__stderr_used` — **Internal**，`#[no_mangle]` 导出以保持符号可见性，仅由 `__stdio_exit` 内部使用；Rust 侧使用 `pub(crate)` 可见性
- `buf` — **Internal**，模块内部静态变量，不对外导出

---

## 内部数据结构

### 1. `buf: [u8; UNGET]`

[Visibility]: Internal — 模块内部静态变量，不对外导出

用于标准错误输出的 8 字节缓冲区。由于 stderr 是无缓冲模式（`buf_size = 0`），该缓冲区仅用于容纳 `UNGET` 预留区（即只用于字符回退操作）。

Rust 侧设计：
- 使用 `static mut BUF: [u8; UNGET]` 或更好的安全抽象
- 对齐和大小与原 C `unsigned char buf[UNGET]` 完全一致

### 2. `__stderr_FILE`

```rust
// 内部 FILE 结构体，字段布局与原 C 完全一致
#[no_mangle]
static __stderr_FILE: FILE = FILE {
    buf: /* buf + UNGET */,
    buf_size: 0,                     // 无缓冲模式
    fd: 2,
    flags: F_PERM | F_NORD,
    lbf: -1,                         // EOF，表示非行缓冲（无缓冲）
    write: Some(__stdio_write),      // 默认写操作
    seek: Some(__stdio_seek),
    close: Some(__stdio_close),
    lock: -1,                        // 免锁模式
    // ... 其余字段保持默认/零值
};
```

[Visibility]: Internal — `#[no_mangle]` 导出符号以维持 C ABI 兼容性，但 Rust 侧通过 `pub(crate)` 控制可见性，不对用户暴露。标准 C 用户通过 `stderr` 指针间接使用。

#### 字段说明

| 字段 | 值 | 含义 |
|------|-----|------|
| `buf` | `&buf[UNGET]` 的地址 | 缓冲区起始于预留 8 字节回退空间之后 |
| `buf_size` | 0 | 无缓冲模式（stderr 默认无缓冲） |
| `fd` | 2 | 文件描述符 2（标准错误输出） |
| `flags` | `F_PERM \| F_NORD` | 永久文件 + 只写（不可读） |
| `lbf` | -1（`EOF`） | 非行缓冲（无缓冲，`EOF = -1`） |
| `write` | `Some(__stdio_write)` | 默认写操作函数指针 |
| `seek` | `Some(__stdio_seek)` | 底层定位操作函数指针 |
| `close` | `Some(__stdio_close)` | 底层关闭操作函数指针 |
| `lock` | -1 | 免锁模式（标准流由 `__stdio_exit` 特殊管理） |

**关键区别**：
- stderr 使用 `__stdio_write`（而非 `__stdout_write`），不检测终端
- stderr 为无缓冲模式（`buf_size = 0`），每次写入直接调用底层写函数
- stderr 的 `lbf = -1`（EOF），与 stdin（0）和 stdout（`'\n'`）均不同

#### 依赖

- `__stdio_write` — 默认 FILE 写操作（见 `__stdio_write` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close` spec）

### 3. `__stderr_used`

```rust
// 指向 __stderr_FILE 的 volatile 指针，用于 __stdio_exit 退出刷新
#[no_mangle]
static __stderr_used: *mut FILE = core::ptr::addr_of_mut!(__stderr_FILE);
```

[Visibility]: Internal — `#[no_mangle]` 导出符号以维持 C ABI 兼容性，Rust 侧 `pub(crate)`，仅由 `__stdio_exit` 使用。

#### Intent

内部哨兵变量。在程序退出时，`__stdio_exit` 函数通过 `__stderr_used` 获取 stderr 的 FILE 指针来执行最终刷新操作。

Rust 侧设计：若链接时未引用 stderr 相关模块，`__stderr_used` 可能通过弱符号机制被替换为 `core::ptr::null_mut()`，`close_file` 会安全地跳过 NULL 指针。

---

## 前置/后置条件

**[Pre-condition]:**
- 无前置条件。`stderr` 在程序启动时由运行时自动初始化。

**[Post-condition]:**
- `stderr` 始终指向有效的 `FILE` 对象（即 `__stderr_FILE`）
- `(*stderr).fd` = 2（标准错误输出文件描述符）
- `(*stderr).flags` 包含 `F_PERM | F_NORD`
- `(*stderr).buf_size` = 0（无缓冲模式）
- `(*stderr).lbf` = -1（EOF，表示非行缓冲）
- `(*stderr).lock` = -1（免锁模式）
- `stderr` 自身为不可变指针，但指向的 `FILE` 对象内容可变（包含缓冲区位置、标志等运行时状态）

**[Error Behavior]:**
- 本模块不产生运行时错误。符号在编译时/链接时静态分配。

---

## 不变量

**[Invariant]:**
- `stderr` 的值（指针地址）在程序整个生命周期内不变，始终指向 `__stderr_FILE`
- `__stderr_FILE` 的生命周期为 `'static`，在程序启动至退出期间有效
- `__stderr_used` 与 `stderr` 指向同一个 `FILE` 对象，用于退出时刷新
- 缓冲区 `buf` 和 `__stderr_FILE` 的字段初始值在编译时确定
- `buf_size` 始终为 0（无缓冲模式）
- `lbf` 始终为 -1（`EOF`），表示无缓冲

---

## 意图

提供标准错误输出流 `stderr` 的全局访问入口。用户程序通过 `stderr` 指针向标准错误输出写入数据。`stderr` 默认无缓冲模式（`buf_size = 0`），每次写入直接执行系统调用。

Rust 侧实现要点：
- `stderr`、`__stderr_FILE`、`__stderr_used` 均使用 `#[no_mangle]` 导出，确保 C 侧链接可见性
- `buf` 为模块内部 `static mut` 或安全等价抽象，不对外导出
- `stderr` 为 `pub` 导出，用户在 `rusl-main` 中通过安全封装访问
- `FILE` 为 `#[repr(C)]` 结构体，所有字段类型和偏移量与原 C `struct _IO_FILE` 布局完全兼容
- `write` 字段使用 `__stdio_write`（默认写），与 stdout 的 `__stdout_write` 不同
- 函数指针字段在 Rust 侧使用 `Option<unsafe extern "C" fn(...)>` 表示可空指针

---

## 依赖图

```
stderr
  ├── stderr (Public) ──> 指向 __stderr_FILE 的不可变指针
  ├── __stderr_FILE (Internal) ──> 直接初始化 #[repr(C)] FILE 结构体
  ├── __stderr_used (Internal) ──> 指向 __stderr_FILE 的可变指针（volatile 语义）
  ├── buf (Internal) ──> [u8; UNGET] 内部缓冲区
  └── (引用函数指针: __stdio_write, __stdio_seek, __stdio_close)
```

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `UNGET` | 8 | `stdio_impl` 模块 | 字符回退预留空间 |
| `F_PERM` | 1 | `stdio_impl` 模块 | 永久流标志（不可被 freopen 重新分配） |
| `F_NORD` | 4 | `stdio_impl` 模块 | 不可读标志 |

---

## [RELY]

- `FILE` 结构体定义 — 字段布局、对齐（见 `stdio_impl` 模块）
- `__stdio_write` — 默认 FILE 写操作（见 `__stdio_write` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close` spec）
- `F_PERM` / `F_NORD` / `UNGET` 常量（见 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
#[no_mangle]
pub static stderr: *mut FILE;
```

本模块保证对外提供上述 ABI 兼容的全局符号：
- `stderr`: 指向标准错误输出 FILE 对象的不可变指针，fd=2，永久且只写，无缓冲模式（`buf_size = 0`）
- 符号 `stderr` 在编译产物（共享库/静态库）中对外可见，外部 C 代码可通过 `extern FILE *const stderr;` 声明后直接访问
- `__stderr_FILE` 和 `__stderr_used` 虽然通过 `#[no_mangle]` 导出，但不对用户暴露，仅供内部模块使用
