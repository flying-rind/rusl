# ftell 模块规约

## 复杂度分级: Level 2

> musl libc 文件流当前位置查询的 Rust 实现。提供 `ftell`、`ftello` 以及内部不加锁版本 `__ftello_unlocked` 和加锁版本 `__ftello`。

---

## 函数接口

```rust
use core::ffi::{c_int, c_long};

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性
// off_t 类型由平台决定（64-bit: c_long, 32-bit+LFS: c_longlong），
// 定义于内部类型模块，此处以 off_t 表示

// ---- 内部符号 ----

unsafe extern "C" fn __ftello_unlocked(f: *mut FILE) -> off_t;
unsafe extern "C" fn __ftello(f: *mut FILE) -> off_t;

// ---- 对外导出符号 ----

unsafe extern "C" fn ftell(f: *mut FILE) -> c_long;

// weak_alias: ftello 是 __ftello 的弱别名，共享同一实现
unsafe extern "C" fn ftello(f: *mut FILE) -> off_t;
```

[Visibility]:
- `__ftello_unlocked` — **Internal (hidden)**，musl 内部实现。由不加锁上下文调用以在不加锁情况下获取文件位置，不对外暴露
- `__ftello` — **Internal (hidden)**，musl 内部加锁版本位置查询函数。是 `ftello` 的主实现（`ftello` 为其弱别名）
- `ftell` — **User**，标准 C 库函数（ISO C / POSIX），声明于 `<stdio.h>`，用户程序可直接调用
- `ftello` — **User**，POSIX 标准函数（需 `_POSIX_C_SOURCE >= 200112L`），弱别名于 `__ftello`

---

## 前置/后置条件

### `__ftello_unlocked`

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，`f->seek` 函数指针有效
- 调用方已持有 `f` 的锁（或不需锁的场景）

**[Post-condition]:**

**Case 1: 成功**
- 返回当前逻辑文件位置（`off_t`，非负值）
- 逻辑位置考虑缓冲区偏移补偿：
  - 读缓冲区预读的数据从内核位置中扣除
  - 写缓冲区未刷新的数据加到内核位置上
- 不修改 `f` 的任何字段（纯查询操作）

**Case 2: 失败 — 底层 seek 失败**
- `f.seek` 返回 `< 0`
- 返回 `-1`（即 `(off_t)(-1)`）
- errno 由底层 `f.seek` 设置

**[Error Behavior]:**
- 底层 seek 失败: return `-1`（errno 由 `f.seek` — 通常是 `__stdio_seek` — 设置，最终源于系统调用 `__lseek` 的 errno 值）

### `__ftello`

**[Pre-condition]:**
- 同 `__ftello_unlocked`
- 调用方不持有 `f` 的锁

**[Post-condition]:**
- `FLOCK(f)` 获取锁成功
- `__ftello_unlocked(f)` 查询当前位置
- `FUNLOCK(f)` 释放锁
- 返回值与 `__ftello_unlocked` 相同

**[Error Behavior]:**
- 与 `__ftello_unlocked` 相同

### `ftell`

**[Pre-condition]:**
- 同 `__ftello`

**[Post-condition]:**

**Case 1: 成功**
- `__ftello(f)` 返回的非负值在 `LONG_MAX` 范围内
- 返回该位置值（类型 `c_long`）

**Case 2: 失败 — 位置超出 long 范围**
- `pos > LONG_MAX`
- `errno` 设置为 `EOVERFLOW`
- 返回 `-1`

**Case 3: 失败 — 底层 seek 失败**
- `__ftello(f)` 返回 `-1`
- 返回 `-1`
- errno 由底层设置

**[Error Behavior]:**
- 位置超出 `LONG_MAX`: errno = `EOVERFLOW`，return `-1`
- 底层 seek 失败: return `-1`（errno 由底层设置）

### `ftello`

**[Pre-condition]:**
- 同 `__ftello`

**[Post-condition]:**
- 与 `__ftello` 完全一致（弱别名，共享同一函数体，返回 `off_t` 类型，无溢出检查）

**[Error Behavior]:**
- 与 `__ftello` 完全相同

---

## 不变量

**[Invariant] — `__ftello_unlocked`:**
- 不修改 `f` 的任何字段（纯查询操作，无副作用）
- 返回的逻辑位置在成功时始终 `>= 0`
- 缓冲区偏移补偿逻辑：
  - `f.rend != null`（有读缓冲区）: 逻辑位置 = 内核位置 - 预读但未消费的字节数（`f.rend - f.rpos`）
  - `f.wbase != null`（有写缓冲区）: 逻辑位置 = 内核位置 + 已写但未刷写的字节数（`f.wpos - f.wbase`）
- 追加模式（`F_APP`）下，若写缓冲区非空，基准 `whence` 切换为 `SEEK_END`，其余逻辑相同

**[Invariant] — `ftello`:**
- `ftello` 和 `__ftello` 返回完全相同的结果，调用的是同一个函数体
- 锁在函数返回前一定被释放

---

## 意图

提供文件流当前位置查询的标准接口。

**`__ftello_unlocked`**: 返回指定 `*mut FILE` 流的当前逻辑位置（从文件起始的字节偏移）。通过底层 `f.seek(f, 0, whence)` 获取内核文件偏移量，再根据缓冲区状态补偿未刷写或已缓冲的数据量。区分读缓冲区和写缓冲区的处理策略。

**`__ftello`**: 获取文件流锁，查询位置，释放锁。是 `ftello` 的主实现。

**`ftell`**: 返回 `c_long` 类型的当前位置。当 `off_t` 结果超出 `LONG_MAX` 时设置 `EOVERFLOW` 并返回 `-1`。

**`ftello`**: 与 `__ftello` 共享同一实现，返回 `off_t` 类型以支持大文件。

典型使用场景：
1. 记录当前文件位置（`ftell`/`ftello`），后续通过 `fseek`/`fseeko` 恢复到该位置
2. 查询文件大小（seek 到末尾后用 `ftell` 获取偏移量）
3. 与 `fgetpos`/`fsetpos` 配合实现位置保存/恢复

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体，字段布局与 musl 的 `FILE` 完全一致
- `off_t` 使用平台对应的类型
- `F_APP`（值 `8`）为模块内部常量
- `LONG_MAX` 来自 `core::ffi::c_long::MAX`
- `EOVERFLOW` 通过 `__errno_location()` 设置
- `FLOCK`/`FUNLOCK` 内部通过调用 `__lockfile`/`__unlockfile` 实现
- `__ftello_unlocked` 为模块私有（`pub(crate)` 或更小可见性）
- `__ftello` 也保持最少可见性，通过 `#[no_mangle]` 导出符号
- `ftello` 作为 `__ftello` 的弱别名，通过 `#[no_mangle]` + 相同函数体实现

## 系统算法

```
__ftello_unlocked(f: *mut FILE) -> off_t:
  1. 获取内核文件偏移量
     base_whence = SEEK_CUR
     if ((*f).flags & F_APP) != 0 and (*f).wpos != (*f).wbase:
       base_whence = SEEK_END    // 追加模式 + 有未刷写数据：用 SEEK_END
     pos = ((*f).seek)(f, 0, base_whence)
     if pos < 0:
       return pos

  2. 缓冲区偏移补偿
     if !(*f).rend.is_null():     // 有读缓冲区数据
       pos += ((*f).rpos as off_t) - ((*f).rend as off_t)
       // 减去预读但未消费的数据量
     else if !(*f).wbase.is_null():  // 有写缓冲区数据
       pos += ((*f).wpos as off_t) - ((*f).wbase as off_t)
       // 加上已写但未刷写的数据量

  3. return pos

__ftello(f: *mut FILE) -> off_t:
  FLOCK(f)
  pos = __ftello_unlocked(f)
  FUNLOCK(f)
  return pos

ftell(f: *mut FILE) -> c_long:
  pos = __ftello(f)                  // off_t 类型
  if pos > LONG_MAX as off_t:
    *__errno_location() = EOVERFLOW
    return -1
  return pos as c_long               // 隐式转换为 c_long

ftello(f: *mut FILE) -> off_t:
  同 __ftello() 的函数体
```

时间复杂度 O(1)（不含底层 `f.seek` 系统调用）。

---

## 依赖图

```
ftell
  └─> __ftello
        ├─> FLOCK / __lockfile       (see __lockfile spec)
        ├─> __ftello_unlocked        (同模块)
        │     └─> f.seek             (函数指针，默认: __stdio_seek)
        │           └─> __lseek      (系统调用)
        └─> FUNLOCK / __unlockfile   (see __lockfile spec)

ftello = weak_alias(__ftello)
```

---

## [RELY]

- `FILE` 结构体定义 — `flags`/`rpos`/`rend`/`wpos`/`wbase`/`seek` 字段布局（见 `stdio_impl` 模块）
- `FLOCK` / `FUNLOCK` — 流锁定/解锁（见 `__lockfile` spec）
- `__stdio_seek` — 底层定位函数，`f.seek` 的默认实现（见 `__stdio_seek` spec）
- `__errno_location` — 设置 errno（见 `__errno_location` spec）
- `off_t` 类型定义 — 平台相关的偏移量类型（见内部类型模块）
- 常量: `F_APP`(8), `SEEK_CUR`(1), `SEEK_END`(2), `LONG_MAX`, `EOVERFLOW`

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn __ftello(f: *mut FILE) -> off_t;
unsafe extern "C" fn ftell(f: *mut FILE) -> c_long;
unsafe extern "C" fn ftello(f: *mut FILE) -> off_t;
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `__ftello`: musl 内部符号，加锁查询文件流当前位置，返回 `off_t` 类型
- `ftell`: 符合 ISO C / POSIX 标准，加锁查询位置，返回 `c_long` 类型（超出 `LONG_MAX` 时设置 `EOVERFLOW`）
- `ftello`: 弱别名于 `__ftello`，符合 POSIX 标准，返回 `off_t` 类型支持大文件

接口不变量：
- 成功时返回当前逻辑文件位置（>= 0），考虑读/写缓冲区偏移补偿
- 追加模式（`F_APP`）下正确使用 `SEEK_END` 作为定位基准
- 纯查询操作，不修改 `FILE` 的任何字段
- 锁在函数返回前一定被释放
