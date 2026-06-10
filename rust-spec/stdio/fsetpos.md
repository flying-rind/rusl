# fsetpos 函数规约

## 复杂度分级: Level 1

> musl libc 文件位置设置的 Rust 实现（ISO C 标准接口）。将文件流位置恢复到先前由 `fgetpos` 保存的位置。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 FILE 布局
// 此处以不透明指针形式呈现，保证 ABI 兼容性
// fpos_t 在 musl 中定义为 c_longlong（i64），
// 此处使用 *const c_longlong 表示，保证 ABI 兼容性

unsafe extern "C" fn fsetpos(f: *mut FILE, pos: *const fpos_t) -> c_int;
// fpos_t 在 Rust 中等价表示为 c_longlong（i64）
```

[Visibility]:
- `fsetpos` — **User**，标准 C 库函数（ISO C），声明于 `<stdio.h>`，用户程序可直接调用

---

## 前置/后置条件

**[Pre-condition]:**
- `f`: 非 NULL 的 `*mut FILE` 指针，`f->seek` 函数指针有效
- `pos`: 非 NULL 的 `*const fpos_t` 指针，指向由合法 `fgetpos` 调用填充的 `fpos_t` 值
- `*pos` 中的值应对应于文件的有效偏移量
- `f` 的底层定位操作可用（文件可定位，如常规文件，不能是管道或终端）

**[Post-condition]:**

**Case 1: 成功**
- 文件位置指示符被设置为 `*(pos as *const c_longlong)` 所表示的绝对偏移量（以 `SEEK_SET` 为基准）
- 内部调用 `__fseeko(f, *(pos as *const c_longlong) as off_t, SEEK_SET)` 执行定位
- 写缓冲区已刷写，读缓冲区被丢弃（由 `__fseeko` 内部 `__fseeko_unlocked` 完成）
- `F_EOF` 标志被清除
- 返回 `0`

**Case 2: 失败**
- `__fseeko` 返回 `-1`
- 返回 `-1`
- errno 由 `__fseeko` 设置（可能包括 `EINVAL`、`ESPIPE`、`EBADF` 等）
- `*pos` 不被修改（`*const` 保证）

**[Error Behavior]:**
- 底层 seek 失败: return `-1`（errno 由 `__fseeko` 设置）
- `*pos` 本身不会被写入或修改

---

## 不变量

**[Invariant]:**
- `*pos` 不被修改（`*const` 语义保证只读）
- `fpos_t` 中存储的值被解释为从文件起始的绝对偏移量（`SEEK_SET`）
- `fsetpos` 与 `fgetpos` 配对使用，保证位置可准确恢复（在文件未被外部截断/修改的前提下）
- 所有操作在锁保护下原子执行（由 `__fseeko` 内部 `FLOCK`/`FUNLOCK` 保证）

---

## 意图

将文件流定位到 `*pos` 所表示的位置。用于配合 `fgetpos` 实现位置保存/恢复。

内部将 `fpos_t` 值解释为绝对偏移量（`SEEK_SET`），直接传递给 `__fseeko` 执行定位。相比 `fseek`/`fsetpos` 的优势在于：
- `fpos_t` 是不透明类型，可以容纳任意大的偏移量（musl 中为 `c_longlong`）
- `fsetpos` 不暴露 `whence` 参数，简化接口
- 与 `fgetpos` 形成标准化配对，提供位置快照/恢复能力

典型使用场景：
1. 在文件处理开始前 `fgetpos` 保存位置，处理完成后 `fsetpos` 恢复
2. 错误恢复：I/O 出错后回滚到之前保存的安全位置
3. 多遍文件处理中记录边界点

Rust 侧实现要点：
- `FILE` 为 `#[repr(C)]` 结构体
- `fpos_t` 在 Rust 中等价表示为 `c_longlong`（`i64`），使用 `*const c_longlong` 作为参数类型
- `__fseeko` 为内部符号（定义于 `fseek` 模块），通过 `extern "C"` 调用
- 实现极为简洁：读取 `*pos` 的值，以 `SEEK_SET` 为基准调用 `__fseeko`
- `SEEK_SET`（值 `0`）为模块内部常量

## 系统算法

```
fsetpos(f: *mut FILE, pos: *const fpos_t) -> c_int:
  // 将 fpos_t 值解释为绝对偏移量，以 SEEK_SET 为基准
  return __fseeko(f, *(pos as *const c_longlong) as off_t, SEEK_SET)
```

时间复杂度 O(1)（不含底层 `__fseeko` 的开销）。

---

## 依赖图

```
fsetpos
  └─> __fseeko                    (see fseek spec)
        ├─> FLOCK / __lockfile    (see __lockfile spec)
        ├─> __fseeko_unlocked     (see fseek spec)
        │     ├─> f.write         (函数指针)
        │     └─> f.seek          (函数指针，默认: __stdio_seek)
        └─> FUNLOCK / __unlockfile (see __lockfile spec)
```

---

## [RELY]

- `__fseeko` — 加锁定位操作（见 `fseek` spec），接受 `off_t` 偏移量和 `whence` 参数
- `FILE` 结构体定义 — 非 NULL 验证及底层函数指针（见 `stdio_impl` 模块）
- `fpos_t` 类型定义 — 等价于 `c_longlong`（见内部类型模块）
- 常量: `SEEK_SET`(0)

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn fsetpos(f: *mut FILE, pos: *const fpos_t) -> c_int;
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `fsetpos`: 符合 ISO C 标准，将文件流定位到 `*pos` 指定的位置
- 成功时返回 `0`，文件位置被设置为 `*pos` 中的绝对偏移量，`F_EOF` 标志被清除
- 失败时返回 `-1`，errno 由底层 seek 设置
- `*pos` 参数为常量，函数不会修改其内容
- 与 `fgetpos` 配对使用可准确恢复文件位置（在文件未被外部修改的前提下）
