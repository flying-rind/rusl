# getc_unlocked 函数规约

## 复杂度分级: Level 1

> musl libc 免锁 FILE 流单字符读取的 Rust 实现。`getc_unlocked` 在 `<stdio.h>` 中通常被定义为宏以提供内联优化，但 musl 同时提供函数实现以支持函数指针等场景。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性

unsafe extern "C" fn getc_unlocked(f: *mut FILE) -> c_int;

// weak_alias: fgetc_unlocked 是 getc_unlocked 的弱别名，POSIX 标准名称
unsafe extern "C" fn fgetc_unlocked(f: *mut FILE) -> c_int;

// weak_alias: _IO_getc_unlocked 是 getc_unlocked 的弱别名，glibc 兼容符号
unsafe extern "C" fn _IO_getc_unlocked(f: *mut FILE) -> c_int;
```

[Visibility]:
- `getc_unlocked` — **User**，`<stdio.h>` POSIX 免锁扩展，用户程序可直接调用。函数符号以括号形式 `(getc_unlocked)` 防止宏展开，确保链接到函数实现。
- `fgetc_unlocked` — **User**，POSIX 标准名称，函数行为与 `getc_unlocked` 完全一致
- `_IO_getc_unlocked` — **Internal**，glibc 兼容别名，不直接对用户暴露，供需要 `_IO_*` 符号的旧代码使用

---

## 前置/后置条件

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，指向已正确初始化且处于读模式（已通过 `__toread` 初始化为读模式）的 `FILE` 结构体
- 调用者已自行获取 `f` 的锁（多线程环境），或确定当前为单线程访问

**[Post-condition]:**
- **Case 1 成功读取字符**
  - 返回读取到的字符（`0`-`255` 的 `c_int` 值，即 `unsigned char` 范围的正值）
  - FILE 流位置前进一个字符

- **Case 2 到达文件末尾**
  - 返回 `EOF`（即 `-1` 的 `c_int` 值）
  - FILE 流设置 `F_EOF` 标志

- **Case 3 读取错误**
  - 返回 `EOF`
  - FILE 流设置 `F_ERR` 标志

**[Error Behavior]:**
- 读取失败时返回 `EOF`，具体错误原因通过 `ferror(f)` 查询
- 可能设置 errno（由底层 `__uflow` 在系统调用失败时设置）

---

## 不变量

**[Invariant]:**
- 不执行加锁操作（调用者负责锁管理）
- 不获取 `f` 的互斥锁（`FLOCK`/`FUNLOCK`）
- `getc_unlocked`、`fgetc_unlocked`、`_IO_getc_unlocked` 三者行为完全一致
- 该函数作为 `getc_unlocked` 宏的函数级回退实现，供函数指针等需要实际函数地址的场景使用

---

## 意图

从 FILE 流 `f` 中读取一个字符，不获取流锁。

**关键差异**：与 `getc(f)` 不同，`getc_unlocked` 函数直接委托给 `getc_unlocked` 宏（定义于 `stdio_impl.h`，内部调用 `__uflow`），不经过 `do_getc` 的加锁逻辑。调用者必须确保在调用此函数前已自行获取 `f` 的锁（通过 `flockfile(f)`），或在单线程环境下使用。

典型使用场景：
1. 批量读取循环中，在 `flockfile(f)` 之后多次调用 `getc_unlocked` 以避免重复加锁开销
2. 函数指针场景，需要获取 `getc_unlocked` 的实际函数地址（而非宏展开）
3. 在已知单线程环境下直接使用，无需加锁

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，与 musl 布局完全一致
- 函数直接委托给 `getc_unlocked` 宏的实现——在 Rust 侧，该宏的语义由内部 `__uflow(f: *mut FILE) -> c_int` 函数承载
- 实际调用链：`getc_unlocked(f)` → 内部 `__uflow(f)` 执行无锁字符读取
- 三个弱别名通过 `#[no_mangle]` + 相同函数体实现，保证链接时解析为同一地址
- 函数体自身不加锁，锁管理完全由调用者负责
- 与 `getc` / `fgetc` / `getchar` 等加锁函数的区别在于：加锁版本通过 `do_getc` 间接调用同一底层 `__uflow`，在调用前后执行加锁/解锁

## 系统算法

```
getc_unlocked(f: *mut FILE) -> c_int:
  return __uflow(f)                     // 委托给无锁底层读取引擎

fgetc_unlocked(f: *mut FILE) -> c_int:
  同 getc_unlocked() 的函数体

_IO_getc_unlocked(f: *mut FILE) -> c_int:
  同 getc_unlocked() 的函数体
```

时间复杂度：O(1)（无阻塞情况），或取决于底层 I/O 操作。

---

## 依赖图

```
getc_unlocked (Public)
  └── __uflow(*mut FILE) -> c_int     (see __uflow spec)

fgetc_unlocked   = weak_alias(getc_unlocked)
_IO_getc_unlocked = weak_alias(getc_unlocked)
```

---

## [RELY]

- `__uflow` — 无锁底层字符读取引擎（见 `__uflow` spec），负责从 FILE 流缓冲区读取单个字符并在缓冲区耗尽时触发底层 I/O 刷新
- `FILE` 结构体定义 — 流状态字段布局（见 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn getc_unlocked(f: *mut FILE) -> c_int;
unsafe extern "C" fn fgetc_unlocked(f: *mut FILE) -> c_int;
unsafe extern "C" fn _IO_getc_unlocked(f: *mut FILE) -> c_int;
```

本模块保证对外提供上述三个 ABI 兼容的函数符号：
- `getc_unlocked`: POSIX 免锁扩展函数，不加锁地从 FILE 流读取一个字符
- `fgetc_unlocked`: 弱别名，POSIX 标准名称，行为与 `getc_unlocked` 完全一致
- `_IO_getc_unlocked`: 弱别名，glibc 兼容，行为与 `getc_unlocked` 完全一致

所有三个符号均不执行内部加锁操作，锁管理由调用者负责。
