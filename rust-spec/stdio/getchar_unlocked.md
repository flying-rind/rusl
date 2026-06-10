# getchar_unlocked 函数规约

## 复杂度分级: Level 1

> musl libc 免锁标准输入单字符读取的 Rust 实现。从 `stdin` 读取一个字符，不获取流锁。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// stdin 为模块内部定义的全局 *mut FILE，指向标准输入流

unsafe extern "C" fn getchar_unlocked() -> c_int;
```

[Visibility]:
- `getchar_unlocked` — **User**，`<stdio.h>` POSIX 免锁扩展（需 `_POSIX_C_SOURCE >= 200112L`），用户程序可直接调用

---

## 前置/后置条件

**[Pre-condition]:**
- `stdin`（全局标准输入流指针）已正确初始化并处于可读状态
- 调用者已自行获取 `stdin` 的锁（多线程环境），或确定当前为单线程访问

**[Post-condition]:**
- **Case 1 成功读取字符**
  - 返回读取到的字符（`0`-`255` 的 `c_int` 值，即 `unsigned char` 范围的正值）
  - `stdin` 流位置前进一个字符

- **Case 2 到达文件末尾**
  - 返回 `EOF`（即 `-1` 的 `c_int` 值）
  - `stdin` 设置 `F_EOF` 标志

- **Case 3 读取错误**
  - 返回 `EOF`
  - `stdin` 设置 `F_ERR` 标志

**[Error Behavior]:**
- 读取失败时返回 `EOF`，具体错误原因通过 `ferror(stdin)` 查询
- 可能设置 errno（由底层 `__uflow` 在系统调用失败时设置）

---

## 不变量

**[Invariant]:**
- 不执行加锁操作（调用者负责锁管理）
- 等价于 `getc_unlocked(stdin)`
- 纯粹的转发代理，无额外状态修改

---

## 意图

从标准输入流 `stdin` 读取一个字符，不获取流锁。

**关键差异**：与 `getchar()` 不同，此函数不调用 `FLOCK(stdin)`/`FUNLOCK(stdin)`。调用者必须确保在调用此函数前已自行获取 `stdin` 的锁，或在单线程环境下使用。

典型使用场景：
1. 配合 `flockfile(stdin)` / `funlockfile(stdin)` 在加锁区间内批量读取
2. 单线程程序中替代 `getchar()` 以消除不必要的锁开销

Rust 侧实现要点：
- `stdin` 为模块内部 `static mut` 全局变量，类型为 `*mut FILE`，由 `__stdin_FILE` 等初始化逻辑赋值
- 函数体等价于 `getc_unlocked(stdin)`，直接委托给 `getc_unlocked` 函数
- 与 `getchar()` 的区别：`getchar()` 展开为 `do_getc(stdin)` → 走加锁路径；`getchar_unlocked()` 展开为 `getc_unlocked(stdin)` → 走免锁路径
- 由于函数签名不包含 FILE 参数（`getchar_unlocked()` 无参数），调用者需确保 `stdin` 已初始化
- 该函数为 `unsafe extern "C"` 以保持 ABI 兼容性，内部实现可以委托给 safe 的 `pub(crate)` 辅助函数

## 系统算法

```
getchar_unlocked() -> c_int:
  return getc_unlocked(stdin)           // 委托给免锁读取，固定从 stdin 读取
```

时间复杂度 O(1)（无阻塞情况），或取决于底层 I/O 操作。

---

## 依赖图

```
getchar_unlocked (Public)
  ├── getc_unlocked(*mut FILE) -> c_int     (see getc_unlocked spec)
  │     └── __uflow(*mut FILE) -> c_int     (see __uflow spec)
  └── stdin (*mut FILE, 全局变量)
```

---

## [RELY]

- `getc_unlocked` — 免锁字符读取函数（见 `getc_unlocked` spec），最终委托 `__uflow`
- `stdin` — 标准输入流全局指针（由 `__stdin_FILE` 初始化，见 `__stdio_init` 相关 spec 或 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn getchar_unlocked() -> c_int;
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `getchar_unlocked`: POSIX 免锁扩展函数，不加锁地从标准输入流 `stdin` 读取一个字符

本函数作为 `getc_unlocked(stdin)` 的薄封装，行为与 `getc_unlocked(stdin)` 完全一致。
