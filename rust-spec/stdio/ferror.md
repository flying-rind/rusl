# ferror 函数规约

## 复杂度分级: Level 1

> musl libc 文件流错误状态查询的 Rust 实现。提供 `ferror`、POSIX 免锁扩展 `ferror_unlocked` 及 glibc 兼容别名 `_IO_ferror_unlocked`。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性

unsafe extern "C" fn ferror(f: *mut FILE) -> c_int;

// weak_alias: ferror_unlocked 是 ferror 的弱别名，共享同一实现
unsafe extern "C" fn ferror_unlocked(f: *mut FILE) -> c_int;

// weak_alias: _IO_ferror_unlocked 是 ferror 的弱别名，glibc 兼容符号
unsafe extern "C" fn _IO_ferror_unlocked(f: *mut FILE) -> c_int;
```

[Visibility]:
- `ferror` — **User**，标准 C 库函数（ISO C），声明于 `<stdio.h>`，用户程序可直接调用
- `ferror_unlocked` — **User**，POSIX 扩展函数，声明于 `<stdio.h>`（需 `_POSIX_C_SOURCE >= 200112L`）
- `_IO_ferror_unlocked` — **Internal**，glibc 兼容别名，不直接对用户暴露，供需要 `_IO_*` 符号的旧代码使用

---

## 前置/后置条件

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，指向已正确初始化的 `FILE` 结构体

**[Post-condition]:**
- 获取 `f` 的互斥锁（`FLOCK(f)`），读取 `f.flags` 字段，释放锁（`FUNLOCK(f)`）
- 若 `f.flags` 中 `F_ERR` 标志位被设置（值 `32`），返回非零值（规范化为 `1`）
- 若 `F_ERR` 未被设置，返回 `0`
- 使用 `!!` 双否定等价操作将位掩码结果规范化为 `0` 或 `1`

**[Error Behavior]:**
- 本函数不产生错误，不设置 errno

---

## 不变量

**[Invariant]:**
- 仅读取 `f.flags`，不修改调用者可见的任何状态
- `ferror`、`ferror_unlocked`、`_IO_ferror_unlocked` 三者行为完全一致，返回相同结果
- 在锁保护下原子地读取标志位，保证线程安全
- `F_ERR` 标志由各类 I/O 操作（如 `fread`、`fwrite`、`fgetc` 等）在发生错误时设置，可通过 `clearerr` 或 `rewind` 清除

---

## 意图

测试文件流的错误指示符。`ferror` 宏（`stdio_impl.h` 中定义）直接读取 `f->flags & F_ERR`，而此函数版本通过 `FLOCK`/`FUNLOCK` 提供线程安全的加锁访问。

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，`flags` 字段与原 C 布局完全一致
- `F_ERR` 为模块内部常量（值 `32`）
- `FLOCK`/`FUNLOCK` 内部通过调用 `__lockfile`/`__unlockfile` 实现，或使用 Rust 的安全锁抽象包装 `FILE` 的锁字段
- 两个弱别名（`ferror_unlocked`、`_IO_ferror_unlocked`）通过 `#[no_mangle]` + 相同函数体实现，保证链接时解析为同一地址

## 系统算法

```
ferror(f: *mut FILE) -> c_int:
  FLOCK(f)                           // 获取 f 的互斥锁
  ret = if (f.flags & F_ERR) != 0 { 1 } else { 0 }  // 读取 ERR 标志并规范化
  FUNLOCK(f)                         // 释放 f 的互斥锁
  return ret

ferror_unlocked(f: *mut FILE) -> c_int:
  同 ferror() 的函数体

_IO_ferror_unlocked(f: *mut FILE) -> c_int:
  同 ferror() 的函数体
```

时间复杂度 O(1)。

---

## 依赖图

```
ferror
  ├─> FLOCK / __lockfile      (see __lockfile spec)
  └─> FUNLOCK / __unlockfile  (see __lockfile spec)

ferror_unlocked = weak_alias(ferror)
_IO_ferror_unlocked = weak_alias(ferror)
```

---

## [RELY]

- `FLOCK` / `FUNLOCK` — 流锁定/解锁，内部依赖 `__lockfile`/`__unlockfile`（见 `__lockfile` spec）
- `FILE` 结构体定义 — `flags` 字段布局，`F_ERR` 常量定义（见 `stdio_impl` 模块）
- `core::ffi::c_int` — Rust core 库提供的 C ABI 兼容整数类型

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn ferror(f: *mut FILE) -> c_int;
unsafe extern "C" fn ferror_unlocked(f: *mut FILE) -> c_int;
unsafe extern "C" fn _IO_ferror_unlocked(f: *mut FILE) -> c_int;
```

本模块保证对外提供上述三个 ABI 兼容的函数符号：
- `ferror`: 线程安全版本，符合 ISO C 标准，加锁检查文件流错误标志
- `ferror_unlocked`: 弱别名，行为与 `ferror` 完全一致
- `_IO_ferror_unlocked`: 弱别名，行为与 `ferror` 完全一致，glibc 兼容

返回值规范化为 `0`（无错误）或 `1`（有错误），严格遵循 C spec 中 `!!` 语义。
