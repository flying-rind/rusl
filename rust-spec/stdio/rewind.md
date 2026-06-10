# rewind 函数规约

## 复杂度分级: Level 1

> musl libc 文件流回绕的 Rust 实现。将文件位置重置到起始并清除错误状态。

---

## 函数接口

```rust
// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性

unsafe extern "C" fn rewind(f: *mut FILE);
```

[Visibility]:
- `rewind` — **User**，标准 C 库函数（ISO C），声明于 `<stdio.h>`，用户程序可直接调用

---

## 前置/后置条件

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，`f->seek` 和 `f->write` 函数指针有效
- 文件流可定位（文件可 seek，如常规文件；管道/终端等不可 seek 文件上调用可能失败但函数不报错）
- 调用方不持有 `f` 的锁

**[Post-condition]:**
- `FLOCK(f)` 获取锁，`FUNLOCK(f)` 释放锁（保证操作的原子性）
- 文件位置指示符被设置为文件起始：
  - 调用 `__fseeko_unlocked(f, 0, SEEK_SET)` 将位置回绕到起始
  - 写缓冲区被刷写（若 `f.wpos != f.wbase`）
  - 读缓冲区被丢弃
  - `f.flags` 中 `F_EOF` 标志被清除（由 `__fseeko_unlocked` 内部完成）
- `f.flags` 中 `F_ERR` 标志被清除
- 函数无返回值——即使底层 seek 失败，调用方也无法通过返回值或 errno 判断操作是否成功（ISO C 标准行为）

**[Error Behavior]:**
- 本函数不返回错误码，不设置 errno
- 即使底层 `__fseeko_unlocked` 定位失败（如管道/终端上调用），函数也不上报错误
- `F_ERR` 标志在函数返回时一定被清除（与 seek 成功/失败无关）

---

## 不变量

**[Invariant]:**
- `F_ERR` 标志在函数返回时一定被清除（无论 seek 是否成功）
- `F_EOF` 标志在成功定位后被清除（由 `__fseeko_unlocked` 内部处理）
- 操作在锁保护下原子执行（不会与其他线程的 I/O 操作交错）
- 函数无返回值——调用方无法通过返回值或 errno 判断操作是否成功（这是 ISO C 标准行为）

---

## 意图

将文件流位置回绕到文件起始，同时清除错误标志 `F_ERR`。

相比 `fseek(f, 0, SEEK_SET)`，`rewind` 额外清除了错误状态，行为等价于：
```
(void)fseek(f, 0, SEEK_SET);
f->flags &= ~F_ERR;
```
但 `rewind` 在一次加锁操作中完成所有步骤，保证原子性。

典型使用场景：
1. 读取完整个文件后回绕到起始重新读取
2. I/O 出错后，使用 `rewind` 清除错误状态并重置位置以便重试
3. 在多遍文件处理中重置到起始位置

与 `fseek` 的重要区别：
- `rewind` 不返回值（`void`），调用方无法判断是否成功
- `rewind` 额外清除 `F_ERR` 标志，`fseek` 不清除 `F_ERR`
- `rewind` 不修改 errno

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，`flags` 字段与原 C 布局完全一致
- `F_ERR`（值 `32`）、`SEEK_SET`（`0`）为模块内部常量
- `FLOCK`/`FUNLOCK` 内部通过调用 `__lockfile`/`__unlockfile` 实现，或使用 Rust 的安全锁抽象包装 `FILE` 的锁字段
- `__fseeko_unlocked` 为同级模块内部符号（定义于 `fseek` 模块），通过 `extern "C"` 调用
- 在持有锁期间依次执行定位和标志清除，保证原子性
- 即使 `__fseeko_unlocked` 返回 `-1`，仍执行 `(*f).flags &= !F_ERR`
- 函数返回类型为 `()`（Rust 的单元类型），对应 C 的 `void`

## 系统算法

```
rewind(f: *mut FILE):
  FLOCK(f)                                    // 获取 f 的互斥锁
  __fseeko_unlocked(f, 0, SEEK_SET)           // 回绕到起始（丢弃返回值）
  (*f).flags &= !F_ERR                         // 清除错误标志（无论 seek 是否成功）
  FUNLOCK(f)                                  // 释放 f 的互斥锁
```

时间复杂度 O(1)（不含底层 `__fseeko_unlocked` 的系统调用开销）。

---

## 依赖图

```
rewind
  ├─> FLOCK / __lockfile           (see __lockfile spec)
  ├─> __fseeko_unlocked            (see fseek spec)
  │     ├─> f.write                (函数指针)
  │     └─> f.seek                 (函数指针，默认: __stdio_seek)
  │           └─> __lseek          (系统调用)
  └─> FUNLOCK / __unlockfile       (see __lockfile spec)
```

---

## [RELY]

- `FLOCK` / `FUNLOCK` — 流锁定/解锁（见 `__lockfile` spec）
- `__fseeko_unlocked` — 不加锁定位（见 `fseek` spec），回绕到文件起始
- `FILE` 结构体定义 — `flags` 字段布局（见 `stdio_impl` 模块）
- 常量: `F_ERR`(32), `SEEK_SET`(0)

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn rewind(f: *mut FILE);
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `rewind`: 符合 ISO C 标准，将文件流位置回绕到起始并清除错误状态
- 函数无返回值（C `void`），调用方无法获知操作是否成功
- `F_ERR` 标志在函数返回时一定被清除（符合 ISO C 标准：`rewind` 等价于 `(void)fseek(f, 0, SEEK_SET)` + `clearerr`）
- 操作在锁保护下原子执行
