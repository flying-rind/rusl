# fgetpos 函数规约

## 复杂度分级: Level 1

> musl libc 文件位置获取的 Rust 实现（ISO C 标准接口）。将与流关联的文件位置指示符的当前值存入 `fpos_t` 对象。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性
// fpos_t 在 musl 中定义为 c_longlong（i64），
// 此处使用 *mut c_longlong 表示，保证 ABI 兼容性

unsafe extern "C" fn fgetpos(f: *mut FILE, pos: *mut fpos_t) -> c_int;
// fpos_t 在 Rust 中等价表示为 c_longlong（i64）
```

[Visibility]:
- `fgetpos` — **User**，标准 C 库函数（ISO C），声明于 `<stdio.h>`，用户程序可直接调用

---

## 前置/后置条件

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，`f->seek` 函数指针有效
- `pos`: 非 NULL 的 `*mut fpos_t` 指针，指向有效的 `fpos_t` 存储空间（至少 `size_of::<c_longlong>()` 字节）
- `f` 的底层定位操作可用（文件可定位，如常规文件，不能是管道或终端）

**[Post-condition]:**

**Case 1: 成功**
- 内部调用 `__ftello(f)` 获取当前 `off_t` 位置
- `*pos` 被写入代表当前逻辑文件位置的 `off_t` 值（以 `c_longlong` 形式存储）
- 返回 `0`

**Case 2: 失败 — 无法获取位置**
- `__ftello(f)` 返回 `< 0`
- `*pos` 未被修改（原子性语义：失败不产生副作用）
- 返回 `-1`
- errno 由 `__ftello` 设置

**Case 3: 失败 — 位置超出 long long 范围**（理论上，musl 目标平台上不发生）
- musl 实现中 `fgetpos` 直接调用 `__ftello` 而非 `ftell`，不进行 `LONG_MAX` 溢出检查
- 若 `off_t` 为 64 位且文件偏移超出 `long long` 范围（> 9EB），可能存在截断问题，但这在 musl 目标平台（`off_t` <= `long long`）上不会发生

**[Error Behavior]:**
- 底层 seek 失败: return `-1`（errno 由 `__ftello` 设置，可能包括 `ESPIPE`、`EBADF` 等）

---

## 不变量

**[Invariant]:**
- `*pos` 在失败时不被修改（原子性：失败不产生副作用）
- `fpos_t` 中存储的值对应于 `__ftello` 的 `off_t` 返回值（未做变换，直接复制）
- 所有操作在锁保护下原子执行（由 `__ftello` 内部 `FLOCK`/`FUNLOCK` 保证）

---

## 意图

获取文件流的当前逻辑位置并存入用户提供的 `fpos_t` 对象。该值后续可通过 `fsetpos` 恢复到同一位置。

相比 `ftell`/`fseek`，`fgetpos`/`fsetpos` 使用不透明类型 `fpos_t` 存储位置，可涵盖任意大的文件偏移（在 musl 中 `fpos_t` 映射为 `c_longlong`）。

典型使用场景：
1. 在对文件进行一系列操作前，调用 `fgetpos` 保存当前位置；出错后通过 `fsetpos` 回滚
2. 在可搜索文件上实现随机访问时标记位置
3. 与 `fsetpos` 配合实现位置快照/恢复

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体
- `fpos_t` 在 Rust 中等价表示为 `c_longlong`（`i64`），使用 `*mut c_longlong` 作为参数类型
- 由于 `fpos_t = c_longlong` 且 `off_t <= c_longlong`（在 musl 目标平台上），`*(pos as *mut c_longlong) = off as c_longlong` 不会发生有损截断
- `__ftello` 为内部符号（定义于 `ftell` 模块），通过 `extern "C"` 调用，内部已处理 `FLOCK`/`FUNLOCK`
- 实现极为简洁：内部调用 `__ftello(f)`，检查返回值，写入 `*pos`，返回结果

## 系统算法

```
fgetpos(f: *mut FILE, pos: *mut fpos_t) -> c_int:
  off = __ftello(f)                      // 获取当前 off_t 位置（内部已加锁）
  if off < 0:                            // 定位失败
    return -1
  *(pos as *mut c_longlong) = off as c_longlong  // 将 off_t 存入 fpos_t
  return 0
```

时间复杂度 O(1)（不含底层 `__ftello` 的系统调用开销）。

---

## 依赖图

```
fgetpos
  └─> __ftello                   (see ftell spec)
        ├─> FLOCK / __lockfile   (see __lockfile spec)
        ├─> __ftello_unlocked    (see ftell spec)
        └─> FUNLOCK / __unlockfile (see __lockfile spec)
```

---

## [RELY]

- `__ftello` — 加锁位置查询（见 `ftell` spec），返回当前 `off_t` 位置
- `FILE` 结构体定义 — 非 NULL 验证及底层函数指针（见 `stdio_impl` 模块）
- `fpos_t` 类型定义 — 等价于 `c_longlong`（见内部类型模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn fgetpos(f: *mut FILE, pos: *mut fpos_t) -> c_int;
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `fgetpos`: 符合 ISO C 标准，获取文件流的当前逻辑位置并存入 `*pos`
- 成功时 `*pos` 被写入位置值，返回 `0`
- 失败时 `*pos` 不被修改，返回 `-1`
- `fpos_t` 中存储的值可直接用于后续 `fsetpos` 调用恢复到同一位置
