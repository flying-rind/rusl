# putchar_unlocked 函数规约

## 复杂度分级: Level 1

> musl libc 免锁标准输出单字符写入的 Rust 实现。将一个字符写入 `stdout`，不获取流锁。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// stdout 为模块内部定义的全局 *mut FILE，指向标准输出流

unsafe extern "C" fn putchar_unlocked(c: c_int) -> c_int;
```

[Visibility]:
- `putchar_unlocked` — **User**，`<stdio.h>` POSIX 免锁扩展（需 `_POSIX_C_SOURCE >= 200112L`），用户程序可直接调用

---

## 前置/后置条件

**[Pre-condition]:**
- `c`: 要写入的字符（以 `c_int` 传递，内部取低 8 位转为 `unsigned char`）
- `stdout`（全局标准输出流指针）已正确初始化并处于可写状态
- 调用者已自行获取 `stdout` 的锁（多线程环境），或确定当前为单线程访问

**[Post-condition]:**
- **Case 1 成功写入**
  - 返回写入的字符（`0`-`255` 的 `c_int` 值，即 `unsigned char` 范围的正值）
  - 字符已写入 `stdout`

- **Case 2 写入错误**
  - 返回 `EOF`（即 `-1` 的 `c_int` 值）
  - `stdout` 设置 `F_ERR` 标志

**[Error Behavior]:**
- 写入失败时返回 `EOF`，具体错误原因通过 `ferror(stdout)` 查询
- 可能设置 errno（由底层 `__overflow` 在系统调用失败时设置）

---

## 不变量

**[Invariant]:**
- 不执行加锁操作（调用者负责锁管理）
- 等价于 `putc_unlocked(c, stdout)`
- 若 `stdout` 为行缓冲模式，写入 `'\n'` 时会触发缓冲区刷出
- 纯粹的转发代理，无额外状态修改

---

## 意图

将字符 `c` 写入标准输出流 `stdout`，不获取流锁。

**关键差异**：与 `putchar(c)` 不同，此函数不调用 `FLOCK(stdout)`/`FUNLOCK(stdout)`。调用者必须确保在调用此函数前已自行获取 `stdout` 的锁，或在单线程环境下使用。

典型使用场景：
1. 配合 `flockfile(stdout)` / `funlockfile(stdout)` 在加锁区间内批量写入
2. 单线程程序中替代 `putchar()` 以消除不必要的锁开销

Rust 侧实现要点：
- `stdout` 为模块内部 `static mut` 全局变量，类型为 `*mut FILE`，由 `__stdout_FILE` 等初始化逻辑赋值
- 函数体等价于 `putc_unlocked(c, stdout)`，直接委托给 `putc_unlocked` 函数
- 与 `putchar(c)` 的区别：`putchar(c)` 展开为 `do_putc(c, stdout)` → 走加锁路径；`putchar_unlocked(c)` 展开为 `putc_unlocked(c, stdout)` → 走免锁路径
- 参数 `c` 在内部被截断为 `unsigned char`（`c as u8`）
- 该函数为 `unsafe extern "C"` 以保持 ABI 兼容性，内部实现可以委托给 safe 的 `pub(crate)` 辅助函数

## 系统算法

```
putchar_unlocked(c: c_int) -> c_int:
  return putc_unlocked(c, stdout)       // 委托给免锁写入，固定写入到 stdout
```

时间复杂度 O(1)（无阻塞情况），或取决于底层 I/O 操作。

---

## 依赖图

```
putchar_unlocked (Public)
  ├── putc_unlocked(c_int, *mut FILE) -> c_int     (see putc_unlocked spec)
  │     └── __overflow(*mut FILE, c_int) -> c_int  (see __overflow spec)
  └── stdout (*mut FILE, 全局变量)
```

---

## [RELY]

- `putc_unlocked` — 免锁字符写入函数（见 `putc_unlocked` spec），最终委托 `__overflow`
- `stdout` — 标准输出流全局指针（由 `__stdout_FILE` 初始化，见 `__stdio_init` 相关 spec 或 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn putchar_unlocked(c: c_int) -> c_int;
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `putchar_unlocked`: POSIX 免锁扩展函数，不加锁地向标准输出流 `stdout` 写入一个字符

本函数作为 `putc_unlocked(c, stdout)` 的薄封装，行为与 `putc_unlocked(c, stdout)` 完全一致。
