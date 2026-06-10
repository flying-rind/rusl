# clearerr 函数规约

## 复杂度分级: Level 1

> musl libc 文件流错误状态清除的 Rust 实现。提供 `clearerr` 和 POSIX 免锁扩展 `clearerr_unlocked`。

---

## 函数接口

```rust
// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性

unsafe extern "C" fn clearerr(f: *mut FILE);

// weak_alias: clearerr_unlocked 是 clearerr 的弱别名，共享同一实现
unsafe extern "C" fn clearerr_unlocked(f: *mut FILE);
```

[Visibility]:
- `clearerr` — **User**，标准 C 库函数（ISO C），声明于 `<stdio.h>`，用户程序可直接调用
- `clearerr_unlocked` — **User**，POSIX 扩展函数，声明于 `<stdio.h>`（需 `_POSIX_C_SOURCE >= 200112L`）

---

## 前置/后置条件

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，指向已正确初始化的 `FILE` 结构体

**[Post-condition]:**
- 获取 `f` 的互斥锁（`FLOCK(f)`），修改 `f.flags` 字段，释放锁（`FUNLOCK(f)`）
- `f.flags` 中 `F_EOF`（值 `16`）和 `F_ERR`（值 `32`）标志位被清除：
  `f.flags &= !(F_EOF | F_ERR)`（Rust 中等价于 `f.flags &= !(F_EOF | F_ERR)`）
- 其他标志位（`F_NOWR`、`F_NORD`、`F_APP`、`F_SVB`、`F_PERM` 等）保持不变
- 调用后 `feof(f)` 和 `ferror(f)` 均返回 `0`
- 无返回值

**[Error Behavior]:**
- 本函数不产生错误，不设置 errno

---

## 不变量

**[Invariant]:**
- 仅修改 `f.flags` 中的 `F_EOF` 和 `F_ERR` 位，不改变其他任何字段（如 `f.fd`、`f.buf`、`f.rpos`、`f.rend` 等）
- `clearerr` 和 `clearerr_unlocked` 行为完全一致
- 操作在锁保护下原子执行，保证线程安全

---

## 意图

清除文件流的文件结束指示符（`F_EOF`）和错误指示符（`F_ERR`）。调用后允许在出错或 EOF 后重试 I/O 操作。

典型使用场景：
1. 在 `fread` 返回 `0` 后区分 EOF 和错误，若为 EOF 则先 `clearerr` 再尝试其他操作
2. I/O 操作失败后，调用 `clearerr` 重置错误状态以便后续操作
3. 在不可定位文件（如管道）上发生 I/O 错误后重置

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，`flags` 字段与原 C 布局完全一致
- `F_EOF`（`16`）和 `F_ERR`（`32`）为模块内部常量
- `FLOCK`/`FUNLOCK` 内部通过调用 `__lockfile`/`__unlockfile` 实现，或使用 Rust 的安全锁抽象包装 `FILE` 的锁字段
- 在持有锁期间，通过 `(*f).flags &= !(F_EOF | F_ERR)` 直接修改标志位
- 弱别名 `clearerr_unlocked` 通过 `#[no_mangle]` + 相同函数体实现，保证链接时解析为同一地址
- POSIX 标准规定 `clearerr_unlocked` 是不加锁版本，但 musl 中两者实现相同（均加锁）

## 系统算法

```
clearerr(f: *mut FILE):
  FLOCK(f)                                    // 获取 f 的互斥锁
  (*f).flags &= !(F_EOF | F_ERR)              // 清除 EOF 和 ERR 标志位
  FUNLOCK(f)                                  // 释放 f 的互斥锁

clearerr_unlocked(f: *mut FILE):
  同 clearerr() 的函数体
```

时间复杂度 O(1)。

---

## 依赖图

```
clearerr
  ├─> FLOCK / __lockfile      (see __lockfile spec)
  └─> FUNLOCK / __unlockfile  (see __lockfile spec)

clearerr_unlocked = weak_alias(clearerr)
```

---

## [RELY]

- `FLOCK` / `FUNLOCK` — 流锁定/解锁，内部依赖 `__lockfile`/`__unlockfile`（见 `__lockfile` spec）
- `FILE` 结构体定义 — `flags` 字段布局，`F_EOF` / `F_ERR` 常量定义（见 `stdio_impl` 模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn clearerr(f: *mut FILE);
unsafe extern "C" fn clearerr_unlocked(f: *mut FILE);
```

本模块保证对外提供上述两个 ABI 兼容的函数符号：
- `clearerr`: 线程安全版本，符合 ISO C 标准，加锁清除文件流 EOF/ERR 标志
- `clearerr_unlocked`: 弱别名，行为与 `clearerr` 完全一致

调用后 `feof(f)` 和 `ferror(f)` 均返回 `0`，其他标志位及 `FILE` 结构体其余字段不受影响。
