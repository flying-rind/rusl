# fseek 模块规约

## 复杂度分级: Level 2

> musl libc 文件流定位操作的 Rust 实现。提供 `fseek`、`fseeko` 以及内部不加锁版本 `__fseeko_unlocked` 和加锁版本 `__fseeko`。

---

## 函数接口

```rust
use core::ffi::{c_int, c_long};

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性
// off_t 类型由平台决定（64-bit: c_long, 32-bit+LFS: c_longlong），
// 定义于内部类型模块，此处以 off_t 表示

// ---- 内部符号 ----

unsafe extern "C" fn __fseeko_unlocked(f: *mut FILE, off: off_t, whence: c_int) -> c_int;
unsafe extern "C" fn __fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int;

// ---- 对外导出符号 ----

unsafe extern "C" fn fseek(f: *mut FILE, off: c_long, whence: c_int) -> c_int;

// weak_alias: fseeko 是 __fseeko 的弱别名，共享同一实现
unsafe extern "C" fn fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int;
```

[Visibility]:
- `__fseeko_unlocked` — **Internal (hidden)**，musl 内部实现。由不加锁上下文调用执行实际定位操作，不对外暴露
- `__fseeko` — **Internal (hidden)**，musl 内部加锁版本定位函数。是 `fseeko` 的主实现（`fseeko` 为其弱别名）。由 `fseek`、`fsetpos` 等调用
- `fseek` — **User**，标准 C 库函数（ISO C / POSIX），声明于 `<stdio.h>`，用户程序可直接调用
- `fseeko` — **User**，POSIX 标准函数（需 `_POSIX_C_SOURCE >= 200112L`），弱别名于 `__fseeko`

---

## 前置/后置条件

### `__fseeko_unlocked`

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，`f->seek` 和 `f->write` 函数指针已由 `fdopen`/`fopen` 正确初始化
- `off`: 相对偏移量
- `whence`: 合法值 `SEEK_SET`（0）、`SEEK_CUR`（1）或 `SEEK_END`（2）
- 调用方已持有 `f` 的锁（或不需锁的场景）

**[Post-condition]:**

**Case 1: 成功**
- `whence` 合法
- 写缓冲区（若 `f.wpos != f.wbase`）已通过 `f.write(f, 0, 0)` 刷写；若刷写失败函数提前返回 `-1`
- 内部写缓冲区指针清零：`f.wpos = f.wbase = f.wend = ptr::null_mut()`
- 底层 `f.seek(f, off, whence)` 调用成功
- 内部读缓冲区指针清零：`f.rpos = f.rend = ptr::null_mut()`
- `f.flags` 中 `F_EOF` 标志被清除
- 返回 `0`

**Case 2: 失败 — whence 非法**
- `errno` 设置为 `EINVAL`
- 返回 `-1`

**Case 3: 失败 — 写缓冲刷写失败**
- `f.write` 返回后 `f.wpos` 为 NULL（表明写入失败）
- 返回 `-1`

**Case 4: 失败 — 底层 seek 失败**
- `f.seek` 返回 `< 0`
- 返回 `-1`
- errno 由底层 seek 设置

**[Error Behavior]:**
- whence 非法: errno = `EINVAL`，return `-1`
- 写缓冲刷写失败: return `-1`（errno 由 `f.write` 设置）
- 底层 seek 失败: return `-1`（errno 由 `f.seek` 设置）

### `__fseeko`

**[Pre-condition]:**
- 同 `__fseeko_unlocked`
- 调用方不持有 `f` 的锁

**[Post-condition]:**
- `FLOCK(f)` 获取锁成功
- `__fseeko_unlocked(f, off, whence)` 执行定位
- `FUNLOCK(f)` 释放锁
- 返回值与 `__fseeko_unlocked` 相同（成功 `0`，失败 `-1`）

**[Error Behavior]:**
- 与 `__fseeko_unlocked` 相同

### `fseek`

**[Pre-condition]:**
- 同 `__fseeko`

**[Post-condition]:**
- 内部委托给 `__fseeko(f, off, whence)`，`c_long` 类型 `off` 隐式转换为 `off_t`
- 返回 `__fseeko` 的返回值

**[Error Behavior]:**
- 与 `__fseeko` 完全相同

### `fseeko`

**[Pre-condition]:**
- 同 `__fseeko`

**[Post-condition]:**
- 与 `__fseeko` 完全一致（弱别名，共享同一函数体）

**[Error Behavior]:**
- 与 `__fseeko` 完全相同

---

## 不变量

**[Invariant] — `__fseeko_unlocked`:**
- 函数成功时：`f.rpos == f.rend == ptr::null_mut()`（读缓冲区被丢弃）且 `f.wpos == f.wbase == f.wend == ptr::null_mut()`（写缓冲区清零）
- `F_EOF` 标志在成功定位后被清除（定位使 EOF 条件失效）
- `SEEK_CUR` 偏移补偿：缓冲区中已读取但未消费的数据量（`f.rend - f.rpos`）从 `off` 中扣除

**[Invariant] — `fseeko`:**
- `fseeko` 和 `__fseeko` 返回完全相同的结果，调用的是同一个函数体
- 锁在函数返回前一定被释放（即使定位失败）

---

## 意图

提供文件流定位操作的标准接口。

**`__fseeko_unlocked`**: 不加锁的文件流定位引擎。处理缓冲区同步（刷新写缓冲、丢弃读缓冲），然后调用底层 `f.seek` 函数指针执行定位。正确处理 `SEEK_CUR` 时缓冲区中未读数据的偏移补偿。

**`__fseeko`**: 获取文件流锁，调用 `__fseeko_unlocked` 执行定位操作，释放锁。是 `fseeko` 的主实现。

**`fseek`**: 为 `c_long` 类型偏移量提供标准文件定位接口，内部委托给 `__fseeko`。

**`fseeko`**: 与 `__fseeko` 共享同一实现，接受 `off_t` 类型偏移量（支持大文件）。

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，字段布局与 musl 的 `FILE` 完全一致
- `off_t` 使用平台对应的类型（通过 `#[cfg]` 条件编译选择 `c_long` 或 `c_longlong`）
- `F_EOF`（值 `16`）、`SEEK_SET`（`0`）、`SEEK_CUR`（`1`）、`SEEK_END`（`2`）为模块内部常量
- `EINVAL` 通过 `__errno_location()` 设置
- `FLOCK`/`FUNLOCK` 内部通过调用 `__lockfile`/`__unlockfile` 实现
- `__fseeko_unlocked` 为模块私有（`pub(crate)` 或更小可见性），仅在本模块及 `rewind` 等紧密耦合模块中可见
- `__fseeko` 也保持最少可见性，通过 `#[no_mangle]` 导出符号供 `fseek`、`fsetpos` 链接
- `fseeko` 作为 `__fseeko` 的弱别名，Rust 侧通过 `#[no_mangle]` + 相同函数体实现，保证链接时解析为同一地址

## 系统算法

```
__fseeko_unlocked(f: *mut FILE, off: off_t, whence: c_int) -> c_int:
  1. 验证 whence ∈ {SEEK_CUR, SEEK_SET, SEEK_END}
     - 不合法: errno = EINVAL, return -1

  2. SEEK_CUR 偏移补偿（缓冲区中已读取但未消耗的数据）
     if whence == SEEK_CUR and !(*f).rend.is_null():
       off -= (*f).rend as isize - (*f).rpos as isize

  3. 刷写写缓冲区
     if (*f).wpos != (*f).wbase:
       ((*f).write)(f, ptr::null(), 0)  // 触发缓冲区刷写
       if (*f).wpos.is_null():          // write 失败
         return -1

  4. 离开写模式
     (*f).wpos = ptr::null_mut()
     (*f).wbase = ptr::null_mut()
     (*f).wend = ptr::null_mut()

  5. 执行底层定位
     if ((*f).seek)(f, off, whence) < 0:
       return -1

  6. 丢弃读缓冲区并清除 EOF 标志
     (*f).rpos = ptr::null_mut()
     (*f).rend = ptr::null_mut()
     (*f).flags &= !F_EOF
     return 0

__fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int:
  FLOCK(f)
  result = __fseeko_unlocked(f, off, whence)
  FUNLOCK(f)
  return result

fseek(f: *mut FILE, off: c_long, whence: c_int) -> c_int:
  return __fseeko(f, off as off_t, whence)

fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int:
  同 __fseeko() 的函数体
```

时间复杂度 O(1)（不含底层 `f.seek` 的开销）。

---

## 依赖图

```
fseek
  └─> __fseeko
        ├─> FLOCK / __lockfile       (see __lockfile spec)
        ├─> __fseeko_unlocked        (同模块)
        │     ├─> f.write            (函数指针，由 fdopen/fopen 初始化)
        │     └─> f.seek             (函数指针，默认: __stdio_seek)
        └─> FUNLOCK / __unlockfile   (see __lockfile spec)

fseeko = weak_alias(__fseeko)
```

---

## [RELY]

- `FILE` 结构体定义 — `wpos`/`wbase`/`wend`/`rpos`/`rend`/`flags`/`write`/`seek` 字段布局（见 `stdio_impl` 模块）
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（见 `__lockfile` spec）
- `__stdio_seek` — 底层定位函数，`f.seek` 的默认实现（见 `__stdio_seek` spec）
- `__errno_location` — 设置 errno（见 `__errno_location` spec）
- `off_t` 类型定义 — 平台相关的偏移量类型（见内部类型模块）
- 常量: `F_EOF`(16), `SEEK_SET`(0), `SEEK_CUR`(1), `SEEK_END`(2), `EINVAL`

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn __fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int;
unsafe extern "C" fn fseek(f: *mut FILE, off: c_long, whence: c_int) -> c_int;
unsafe extern "C" fn fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int;
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `__fseeko`: musl 内部符号，加锁执行文件流定位，接受 `off_t` 偏移量
- `fseek`: 符合 ISO C / POSIX 标准，加锁定位，接受 `c_long` 偏移量
- `fseeko`: 弱别名于 `__fseeko`，符合 POSIX 标准，接受 `off_t` 偏移量支持大文件

接口不变量：
- 成功时读/写缓冲区被清零，`F_EOF` 标志被清除，返回 `0`
- whence 非法时 `errno = EINVAL`，返回 `-1`
- 底层 seek 失败时 errno 由底层设置，返回 `-1`
- 锁在函数返回前一定被释放
