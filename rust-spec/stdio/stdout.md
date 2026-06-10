# stdout 规约

## 复杂度分级: Level 1

> musl libc 标准输出流全局变量的 Rust 实现。包含 `stdout`（对外导出）、`__stdout_FILE`（内部实现）和 `__stdout_used`（内部哨兵变量）。

---

## 全局接口

```rust
// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 struct _IO_FILE
// 此处以不透明指针形式呈现，保证 ABI 兼容性

// stdout: 指向标准输出 FILE 对象的常量指针
// 对应 C 的 FILE *const stdout = &__stdout_FILE;
#[no_mangle]
pub static stdout: *mut FILE;
```

[Visibility]:
- `stdout` — **User**，标准 C 库全局变量（ISO C），声明于 `<stdio.h>`，用户程序通过 `stdout` 宏直接使用
- `__stdout_FILE` — **Internal**，`#[no_mangle]` 导出以保持符号可见性，但不对用户暴露；Rust 侧使用 `pub(crate)` 可见性
- `__stdout_used` — **Internal**，`#[no_mangle]` 导出以保持符号可见性，仅由 `__stdio_exit` 内部使用；Rust 侧使用 `pub(crate)` 可见性
- `buf` — **Internal**，模块内部静态变量，不对外导出

---

## 内部数据结构

### 1. `buf: [u8; BUFSIZ + UNGET]`

[Visibility]: Internal — 模块内部静态变量，不对外导出

标准输出的内部缓冲区。大小为 `BUFSIZ + UNGET`（1024 + 8 = 1032 字节），其中：
- 后 1024 字节（`buf[UNGET..]`）为实际写缓冲区（`buf_size = 1024`）
- 前 8 字节为 `UNGET` 字符回退预留区

Rust 侧设计：
- 使用 `static mut BUF: [u8; BUFSIZ + UNGET]` 或更好的安全抽象
- 对齐和大小与原 C `unsigned char buf[BUFSIZ+UNGET]` 完全一致

### 2. `__stdout_FILE`

```rust
// 内部 FILE 结构体，字段布局与原 C 完全一致
#[no_mangle]
static __stdout_FILE: FILE = FILE {
    buf: /* buf + UNGET */,
    buf_size: BUFSIZ,
    fd: 1,
    flags: F_PERM | F_NORD,
    lbf: b'\n' as c_int,             // 行缓冲模式（遇 '\n' 自动刷新）
    write: Some(__stdout_write),     // stdout 专用写函数
    seek: Some(__stdio_seek),
    close: Some(__stdio_close),
    lock: -1,                        // 免锁模式
    // ... 其余字段保持默认/零值
};
```

[Visibility]: Internal — `#[no_mangle]` 导出符号以维持 C ABI 兼容性，但 Rust 侧通过 `pub(crate)` 控制可见性，不对用户暴露。标准 C 用户通过 `stdout` 指针间接使用。

#### 字段说明

| 字段 | 值 | 含义 |
|------|-----|------|
| `buf` | `&buf[UNGET]` 的地址 | 缓冲区起始于预留 8 字节回退空间之后 |
| `buf_size` | `BUFSIZ` = 1024 | BUFSIZ 大小的写缓冲区 |
| `fd` | 1 | 文件描述符 1（标准输出） |
| `flags` | `F_PERM \| F_NORD` | 永久文件 + 只写（不可读） |
| `lbf` | `b'\n'`（即 `0x0A`） | 行缓冲模式（遇到换行符自动刷新） |
| `write` | `Some(__stdout_write)` | stdout 专用写函数（检测终端行缓冲） |
| `seek` | `Some(__stdio_seek)` | 底层定位操作函数指针 |
| `close` | `Some(__stdio_close)` | 底层关闭操作函数指针 |
| `lock` | -1 | 免锁模式（标准流由 `__stdio_exit` 特殊管理） |

**关键区别**：
- stdout 使用 `__stdout_write`（而非 `__stdio_write`），该函数在写入前检测文件描述符是否为终端，若是终端则强制使用 `'\n'` 行缓冲
- stdout 的行缓冲标志 `lbf = b'\n'`，表示遇到换行符时自动刷新

#### 依赖

- `__stdout_write` — stdout 专用写操作（见 `__stdout_write` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close` spec）

### 3. `__stdout_used`

```rust
// 指向 __stdout_FILE 的 volatile 指针，用于 __stdio_exit 退出刷新
#[no_mangle]
static __stdout_used: *mut FILE = core::ptr::addr_of_mut!(__stdout_FILE);
```

[Visibility]: Internal — `#[no_mangle]` 导出符号以维持 C ABI 兼容性，Rust 侧 `pub(crate)`，仅由 `__stdio_exit` 使用。

#### Intent

内部哨兵变量。在程序退出时，`__stdio_exit` 函数通过 `__stdout_used` 获取 stdout 的 FILE 指针来执行最终刷新操作。

Rust 侧设计：若链接时未引用 stdout 相关模块，`__stdout_used` 可能通过弱符号机制被替换为 `core::ptr::null_mut()`，`close_file` 会安全地跳过 NULL 指针。

---

## 前置/后置条件

**[Pre-condition]:**
- 无前置条件。`stdout` 在程序启动时由运行时自动初始化。

**[Post-condition]:**
- `stdout` 始终指向有效的 `FILE` 对象（即 `__stdout_FILE`）
- `(*stdout).fd` = 1（标准输出文件描述符）
- `(*stdout).flags` 包含 `F_PERM | F_NORD`
- `(*stdout).lbf` = `b'\n'`（行缓冲模式）
- `(*stdout).lock` = -1（免锁模式）
- `stdout` 自身为不可变指针，但指向的 `FILE` 对象内容可变

**[Error Behavior]:**
- 本模块不产生运行时错误。符号在编译时/链接时静态分配。

---

## 不变量

**[Invariant]:**
- `stdout` 的值（指针地址）在程序整个生命周期内不变，始终指向 `__stdout_FILE`
- `__stdout_FILE` 的生命周期为 `'static`，在程序启动至退出期间有效
- `__stdout_used` 与 `stdout` 指向同一个 `FILE` 对象，用于退出时刷新
- 缓冲区 `buf` 和 `__stdout_FILE` 的字段初始值在编译时确定
- `buf_size` 始终为 1024（`BUFSIZ`）
- `lbf` 始终为 `b'\n'`，表示行缓冲模式

---

## 意图

提供标准输出流 `stdout` 的全局访问入口。用户程序通过 `stdout` 指针向标准输出写入数据。`stdout` 默认行缓冲模式（`lbf = b'\n'`）。

Rust 侧实现要点：
- `stdout`、`__stdout_FILE`、`__stdout_used` 均使用 `#[no_mangle]` 导出，确保 C 侧链接可见性
- `buf` 为模块内部 `static mut` 或安全等价抽象，不对外导出
- `stdout` 为 `pub` 导出，用户在 `rusl-main` 中通过安全封装访问
- `FILE` 为 `#[repr(C)]` 结构体，所有字段类型和偏移量与原 C `struct _IO_FILE` 布局完全兼容
- `write` 字段使用 `__stdout_write` 而非 `__stdio_write`，是 stdout 与 stderr 的关键区别
- 函数指针字段在 Rust 侧使用 `Option<unsafe extern "C" fn(...)>` 表示可空指针

---

## 依赖图

```
stdout
  ├── stdout (Public) ──> 指向 __stdout_FILE 的不可变指针
  ├── __stdout_FILE (Internal) ──> 直接初始化 #[repr(C)] FILE 结构体
  ├── __stdout_used (Internal) ──> 指向 __stdout_FILE 的可变指针（volatile 语义）
  ├── buf (Internal) ──> [u8; BUFSIZ + UNGET] 内部缓冲区
  └── (引用函数指针: __stdout_write, __stdio_seek, __stdio_close)
```

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `BUFSIZ` | 1024 | `stdio_impl` 模块 | 默认缓冲区大小 |
| `UNGET` | 8 | `stdio_impl` 模块 | 字符回退预留空间 |
| `F_PERM` | 1 | `stdio_impl` 模块 | 永久流标志 |
| `F_NORD` | 4 | `stdio_impl` 模块 | 不可读标志（只写） |

---

## [RELY]

- `FILE` 结构体定义 — 字段布局、对齐（见 `stdio_impl` 模块）
- `__stdout_write` — stdout 专用写操作（见 `__stdout_write` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close` spec）
- `F_PERM` / `F_NORD` / `BUFSIZ` / `UNGET` 常量（见 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
#[no_mangle]
pub static stdout: *mut FILE;
```

本模块保证对外提供上述 ABI 兼容的全局符号：
- `stdout`: 指向标准输出 FILE 对象的不可变指针，fd=1，永久且只写，行缓冲模式（`lbf = b'\n'`）
- 符号 `stdout` 在编译产物（共享库/静态库）中对外可见，外部 C 代码可通过 `extern FILE *const stdout;` 声明后直接访问
- `__stdout_FILE` 和 `__stdout_used` 虽然通过 `#[no_mangle]` 导出，但不对用户暴露，仅供内部模块使用
