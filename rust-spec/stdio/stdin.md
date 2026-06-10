# stdin 规约

## 复杂度分级: Level 1

> musl libc 标准输入流全局变量的 Rust 实现。包含 `stdin`（对外导出）、`__stdin_FILE`（内部实现）和 `__stdin_used`（内部哨兵变量）。

---

## 全局接口

```rust
// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 struct _IO_FILE
// 此处以不透明指针形式呈现，保证 ABI 兼容性

// stdin: 指向标准输入 FILE 对象的常量指针
// 对应 C 的 FILE *const stdin = &__stdin_FILE;
#[no_mangle]
pub static stdin: *mut FILE;
```

[Visibility]:
- `stdin` — **User**，标准 C 库全局变量（ISO C），声明于 `<stdio.h>`，用户程序通过 `stdin` 宏直接使用
- `__stdin_FILE` — **Internal**，`#[no_mangle]` 导出以保持符号可见性，但不对用户暴露；Rust 侧使用 `pub(crate)` 可见性
- `__stdin_used` — **Internal**，`#[no_mangle]` 导出以保持符号可见性，仅由 `__stdio_exit` 内部使用；Rust 侧使用 `pub(crate)` 可见性
- `buf` — **Internal**，模块内部静态变量，不对外导出

---

## 内部数据结构

### 1. `buf: [u8; BUFSIZ + UNGET]`

[Visibility]: Internal — 模块内部静态变量，不对外导出

标准输入的内部缓冲区。大小为 `BUFSIZ + UNGET`（1024 + 8 = 1032 字节），其中：
- 后 1024 字节（`buf[UNGET..]`）为实际读缓冲区（`buf_size = 1024`）
- 前 8 字节为 `UNGET` 字符回退预留区

Rust 侧设计：
- 使用 `static mut BUF: [u8; BUFSIZ + UNGET]` 或更好的安全抽象（如 `UnsafeCell<[u8; BUFSIZ + UNGET]>`）
- 对齐和大小与原 C `unsigned char buf[BUFSIZ+UNGET]` 完全一致

### 2. `__stdin_FILE`

```rust
// 内部 FILE 结构体，字段布局与原 C 完全一致
#[no_mangle]
static __stdin_FILE: FILE = FILE {
    buf: /* buf + UNGET */,
    buf_size: BUFSIZ,
    fd: 0,
    flags: F_PERM | F_NOWR,
    lbf: 0,                          // 非行缓冲（全缓冲），默认 0
    read: Some(__stdio_read),
    seek: Some(__stdio_seek),
    close: Some(__stdio_close),
    lock: -1,                        // 免锁模式
    // ... 其余字段保持默认/零值
};
```

[Visibility]: Internal — `#[no_mangle]` 导出符号以维持 C ABI 兼容性，但 Rust 侧通过 `pub(crate)` 控制可见性，不对用户暴露。标准 C 用户通过 `stdin` 指针间接使用。

#### 字段说明

| 字段 | 值 | 含义 |
|------|-----|------|
| `buf` | `&buf[UNGET]` 的地址 | 缓冲区起始于预留 8 字节回退空间之后 |
| `buf_size` | `BUFSIZ` = 1024 | BUFSIZ 大小的读缓冲区 |
| `fd` | 0 | 文件描述符 0（标准输入） |
| `flags` | `F_PERM \| F_NOWR` | 永久文件 + 不可写 |
| `lbf` | 0 | 非行缓冲模式（全缓冲，`stdin` 默认行为） |
| `read` | `Some(__stdio_read)` | 底层读操作函数指针 |
| `seek` | `Some(__stdio_seek)` | 底层定位操作函数指针 |
| `close` | `Some(__stdio_close)` | 底层关闭操作函数指针 |
| `lock` | -1 | 免锁模式（标准流由 `__stdio_exit` 特殊管理） |

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，函数指针字段使用 `Option<unsafe extern "C" fn(...)>` 表示，`None` 等效于 C 的 NULL 函数指针
- `lock: -1` 使用 `c_int` 类型，表示标准流无需显式锁定
- `lbf: 0` 表示非行缓冲（`stdin` 默认全缓冲，与 `stdout` 的 `'\n'` 行缓冲不同）

#### 依赖

- `__stdio_read` — 默认 FILE 读操作（见 `__stdio_read` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close` spec）

### 3. `__stdin_used`

```rust
// 指向 __stdin_FILE 的 volatile 指针，用于 __stdio_exit 退出刷新
#[no_mangle]
static __stdin_used: *mut FILE = core::ptr::addr_of_mut!(__stdin_FILE);
```

[Visibility]: Internal — `#[no_mangle]` 导出符号以维持 C ABI 兼容性，Rust 侧 `pub(crate)`，仅由 `__stdio_exit` 使用。

#### Intent

内部哨兵变量。在程序退出时，`__stdio_exit` 函数通过 `__stdin_used` 获取 stdin 的 FILE 指针来执行最终刷新操作。

Rust 侧设计：若链接时未引用 stdin 相关模块，`__stdin_used` 可能通过弱符号机制被替换为 `core::ptr::null_mut()`（零值），`close_file` 会安全地跳过 NULL 指针。

---

## 前置/后置条件

**[Pre-condition]:**
- 无前置条件。`stdin` 在程序启动时由运行时自动初始化。

**[Post-condition]:**
- `stdin` 始终指向有效的 `FILE` 对象（即 `__stdin_FILE`）
- `(*stdin).fd` = 0（标准输入文件描述符）
- `(*stdin).flags` 包含 `F_PERM | F_NOWR`
- `(*stdin).lock` = -1（免锁模式）
- `stdin` 自身为不可变指针（Rust 中 `static` 无 `mut`，指针绑定不可变），但指向的 `FILE` 对象内容可变（缓冲区位置、文件偏移量等运行时状态）

**[Error Behavior]:**
- 本模块不产生运行时错误。符号在编译时/链接时静态分配。

---

## 不变量

**[Invariant]:**
- `stdin` 的值（指针地址）在程序整个生命周期内不变，始终指向 `__stdin_FILE`
- `__stdin_FILE` 的生命周期为 `'static`，在程序启动至退出期间有效
- `__stdin_used` 与 `stdin` 指向同一个 `FILE` 对象，用于退出时刷新
- 缓冲区 `buf` 和 `__stdin_FILE` 的字段初始值在编译时确定，程序运行期间不被替换
- `buf_size` 始终为 1024（`BUFSIZ`）

---

## 意图

提供标准输入流 `stdin` 的全局访问入口。用户程序通过 `stdin` 指针读取标准输入数据。`stdin` 默认全缓冲模式（`buf_size = BUFSIZ`），`lbf` 字段默认 0（非行缓冲）。

Rust 侧实现要点：
- `stdin`、`__stdin_FILE`、`__stdin_used` 均使用 `#[no_mangle]` 导出，确保 C 侧链接可见性
- `buf` 为模块内部 `static mut` 或安全等价抽象，不对外导出
- `stdin` 为 `pub` 导出，用户在 `rusl-main` 中通过安全封装访问
- `FILE` 为 `#[repr(C)]` 结构体，所有字段类型和偏移量与原 C `struct _IO_FILE` 布局完全兼容
- 函数指针字段在 Rust 侧使用 `Option<unsafe extern "C" fn(...)>` 表示可空指针

---

## 依赖图

```
stdin
  ├── stdin (Public) ──> 指向 __stdin_FILE 的不可变指针
  ├── __stdin_FILE (Internal) ──> 直接初始化 #[repr(C)] FILE 结构体
  ├── __stdin_used (Internal) ──> 指向 __stdin_FILE 的可变指针（volatile 语义）
  ├── buf (Internal) ──> [u8; BUFSIZ + UNGET] 内部缓冲区
  └── (引用函数指针: __stdio_read, __stdio_seek, __stdio_close)
```

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `BUFSIZ` | 1024 | `stdio_impl` 模块 | 默认缓冲区大小 |
| `UNGET` | 8 | `stdio_impl` 模块 | 字符回退预留空间 |
| `F_PERM` | 1 | `stdio_impl` 模块 | 永久流标志 |
| `F_NOWR` | 8 | `stdio_impl` 模块 | 不可写标志 |

---

## [RELY]

- `FILE` 结构体定义 — 字段布局、对齐（见 `stdio_impl` 模块）
- `__stdio_read` — 默认 FILE 读操作（见 `__stdio_read` spec）
- `__stdio_seek` — 默认 FILE 定位操作（见 `__stdio_seek` spec）
- `__stdio_close` — 默认 FILE 关闭操作（见 `__stdio_close` spec）
- `F_PERM` / `F_NOWR` / `BUFSIZ` / `UNGET` 常量（见 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
#[no_mangle]
pub static stdin: *mut FILE;
```

本模块保证对外提供上述 ABI 兼容的全局符号：
- `stdin`: 指向标准输入 FILE 对象的不可变指针，fd=0，永久且不可写，全缓冲模式
- 符号 `stdin` 在编译产物（共享库/静态库）中对外可见，外部 C 代码可通过 `extern FILE *const stdin;` 声明后直接访问
- `__stdin_FILE` 和 `__stdin_used` 虽然通过 `#[no_mangle]` 导出，但不对用户暴露，仅供内部模块使用
