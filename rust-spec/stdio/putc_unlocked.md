# putc_unlocked 函数规约

## 复杂度分级: Level 1

> musl libc 免锁 FILE 流单字符写入的 Rust 实现。`putc_unlocked` 在 `<stdio.h>` 中通常被定义为宏以提供内联优化，但 musl 同时提供函数实现以支持函数指针等场景。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性

unsafe extern "C" fn putc_unlocked(c: c_int, f: *mut FILE) -> c_int;

// weak_alias: fputc_unlocked 是 putc_unlocked 的弱别名，POSIX 标准名称
unsafe extern "C" fn fputc_unlocked(c: c_int, f: *mut FILE) -> c_int;

// weak_alias: _IO_putc_unlocked 是 putc_unlocked 的弱别名，glibc 兼容符号
unsafe extern "C" fn _IO_putc_unlocked(c: c_int, f: *mut FILE) -> c_int;
```

[Visibility]:
- `putc_unlocked` — **User**，`<stdio.h>` POSIX 免锁扩展，用户程序可直接调用。函数符号以括号形式 `(putc_unlocked)` 防止宏展开，确保链接到函数实现。
- `fputc_unlocked` — **User**，POSIX 标准名称，函数行为与 `putc_unlocked` 完全一致
- `_IO_putc_unlocked` — **Internal**，glibc 兼容别名，不直接对用户暴露，供需要 `_IO_*` 符号的旧代码使用

---

## 前置/后置条件

**[Pre-condition]:**
- `c`: 要写入的字符（以 `c_int` 传递，内部取低 8 位转为 `unsigned char`）
- `f`: 非 NULL 的 `*mut FILE` 指针，指向已正确初始化且处于写模式（已通过 `__towrite` 初始化为写模式）的 `FILE` 结构体
- 调用者已自行获取 `f` 的锁（多线程环境），或确定当前为单线程访问

**[Post-condition]:**
- **Case 1 成功写入**
  - 返回写入的字符（`0`-`255` 的 `c_int` 值，即 `unsigned char` 范围的正值）
  - 字符已写入流缓冲区

- **Case 2 写入错误**
  - 返回 `EOF`（即 `-1` 的 `c_int` 值）
  - FILE 流设置 `F_ERR` 标志

**[Error Behavior]:**
- 写入失败时返回 `EOF`，具体错误原因通过 `ferror(f)` 查询
- 可能设置 errno（由底层 `__overflow` 在系统调用失败时设置）

---

## 不变量

**[Invariant]:**
- 不执行加锁操作（调用者负责锁管理）
- 不获取 `f` 的互斥锁（`FLOCK`/`FUNLOCK`）
- `putc_unlocked`、`fputc_unlocked`、`_IO_putc_unlocked` 三者行为完全一致
- 该函数作为 `putc_unlocked` 宏的函数级回退实现，供函数指针等需要实际函数地址的场景使用

---

## 意图

将字符 `c` 写入 FILE 流 `f`，不获取流锁。

**关键差异**：与 `putc(c, f)` 不同，`putc_unlocked` 函数直接委托给 `putc_unlocked` 宏（定义于 `stdio_impl.h`，内部调用 `__overflow`），不经过 `do_putc` 的加锁逻辑。调用者必须确保在调用此函数前已自行获取 `f` 的锁（通过 `flockfile(f)`），或在单线程环境下使用。

典型使用场景：
1. 批量写入循环中，在 `flockfile(f)` 之后多次调用 `putc_unlocked` 以避免重复加锁开销
2. 函数指针场景，需要获取 `putc_unlocked` 的实际函数地址（而非宏展开）
3. 在已知单线程环境下直接使用，无需加锁

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，与 musl 布局完全一致
- 函数直接委托给 `putc_unlocked` 宏的实现——在 Rust 侧，该宏的语义由内部 `__overflow(f: *mut FILE, c: c_int) -> c_int` 函数承载
- 实际调用链：`putc_unlocked(c, f)` → 内部 `__overflow(f, c)` 执行无锁字符写入
- 参数 `c` 在内部被截断为 `unsigned char`（`c as u8`）
- 三个弱别名通过 `#[no_mangle]` + 相同函数体实现，保证链接时解析为同一地址
- 函数体自身不加锁，锁管理完全由调用者负责
- 与 `putc` / `fputc` / `putchar` 等加锁函数的区别在于：加锁版本通过 `do_putc` 间接调用同一底层 `__overflow`，在调用前后执行加锁/解锁

## 系统算法

```
putc_unlocked(c: c_int, f: *mut FILE) -> c_int:
  return __overflow(f, c as u8)         // 委托给无锁底层写入引擎

fputc_unlocked(c: c_int, f: *mut FILE) -> c_int:
  同 putc_unlocked() 的函数体

_IO_putc_unlocked(c: c_int, f: *mut FILE) -> c_int:
  同 putc_unlocked() 的函数体
```

时间复杂度：O(1)（无阻塞情况），或取决于底层 I/O 操作。

---

## 依赖图

```
putc_unlocked (Public)
  └── __overflow(*mut FILE, c_int) -> c_int     (see __overflow spec)

fputc_unlocked   = weak_alias(putc_unlocked)
_IO_putc_unlocked = weak_alias(putc_unlocked)
```

---

## [RELY]

- `__overflow` — 无锁底层字符写入引擎（见 `__overflow` spec），负责将单个字符写入 FILE 流缓冲区并在缓冲区满时触发底层 I/O 刷新
- `FILE` 结构体定义 — 流状态字段布局（见 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn putc_unlocked(c: c_int, f: *mut FILE) -> c_int;
unsafe extern "C" fn fputc_unlocked(c: c_int, f: *mut FILE) -> c_int;
unsafe extern "C" fn _IO_putc_unlocked(c: c_int, f: *mut FILE) -> c_int;
```

本模块保证对外提供上述三个 ABI 兼容的函数符号：
- `putc_unlocked`: POSIX 免锁扩展函数，不加锁地向 FILE 流写入一个字符
- `fputc_unlocked`: 弱别名，POSIX 标准名称，行为与 `putc_unlocked` 完全一致
- `_IO_putc_unlocked`: 弱别名，glibc 兼容，行为与 `putc_unlocked` 完全一致

所有三个符号均不执行内部加锁操作，锁管理由调用者负责。
