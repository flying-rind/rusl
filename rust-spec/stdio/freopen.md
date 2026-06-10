# freopen 函数规约

## 复杂度分级: Level 1

> musl libc 标准库文件重定向函数。将已有 `FILE` 流重定向到新文件路径，或修改已打开文件的访问模式。

---

## 函数接口

```rust
use core::ffi::{c_int, c_char};

// FILE 为 opaque 类型（定义同 fclose.rs spec）
#[repr(C)]
pub struct FILE { _private: [u8; 0] }

/// 将已有 FILE 流重定向。
/// - filename: 新文件路径（可为 NULL，此时只修改当前 fd 的模式）
/// - mode: 新模式字符串
/// - f: 已打开的 FILE 流指针
/// 成功返回 f，失败返回 NULL（原始 f 被关闭）。
unsafe extern "C" fn freopen(
    filename: *const c_char,
    mode: *const c_char,
    f: *mut FILE,
) -> *mut FILE;
```

[Visibility]: `freopen` 声明于 `<stdio.h>`，是用户可直接调用的标准 C 库函数。在编译产物中以 `#[no_mangle]` 导出 `freopen` 符号，必须保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `mode`: 有效的模式字符串，首字符为 `'r'`、`'w'` 或 `'a'`
- `f`: 一个有效的已打开 `*mut FILE` 指针（不能为 `NULL`）
- 若 `filename` 非 `NULL`: 该路径必须为有效文件路径
- 若 `filename` 为 `NULL`: 只修改 `f` 对应文件描述符的访问模式标志（通过 `fcntl`）

**[Post-condition]:**

- **Case 1: filename 非 NULL 且操作成功**
  - `fflush(f)` 被调用以刷新当前缓冲区
  - 通过 `fopen(filename, mode)` 创建一个新的 `FILE` 对象 `f2`
  - 若 `f2.fd == f.fd`，设 `f2.fd = -1`（防止关闭 `f2` 时误关相同 fd）
  - 否则调用 `__dup3(f2.fd, f.fd, fl & O_CLOEXEC)` 将新文件描述符复制到 `f` 的文件描述符
  - 将 `f2` 的操作属性复制到 `f`：`flags`（保留 `F_PERM`）、`read`、`write`、`seek`、`close`
  - `fclose(f2)` 释放临时 `FILE` 对象
  - 重置 `f.mode = 0` 和 `f.locale = 0`
  - 返回 `f`

- **Case 2: filename 为 NULL 且操作成功**
  - `fflush(f)` 刷新缓冲区
  - 若 mode 含 `'e'`，设置 close-on-exec 标志
  - 调用 `fcntl(f.fd, F_SETFL, fl)` 修改文件描述符的访问模式
  - 返回 `f`

- **Case 3: 操作失败**
  - 关闭原始 `f`（调用 `fclose(f)`）
  - 返回 `NULL`

**[Error Behavior]:**

| 条件 | 行为 |
|------|------|
| mode 首字符不合法 | `fopen` 内部设置 `errno = EINVAL`，关闭 `f` |
| `filename` 非 `NULL` 且文件打开失败 | `fopen` 返回 `NULL`，关闭 `f` |
| `__dup3` 失败 | 关闭 `f2`，再关闭 `f` |
| `filename` 为 `NULL` 且 `fcntl` 失败 | 关闭 `f` |

---

### 不变量

**[Invariant]:**
- 在任何条件下，操作最终不会泄露文件描述符或 `FILE` 对象
- `F_PERM` 标志始终保留（若原始流是一个永久流如 stdout）
- 失败时原始 `f` 被关闭（musl 行为：与 glibc 不同，glibc 失败时保留原始 `f`）

---

### 意图

将一个已存在的 `FILE` 流重定向到另一个文件或修改其模式。函数首先刷新 `f` 的缓冲区并关闭其当前关联，然后将 `f` 与 `filename`（若提供）的新文件关联，或将 `f` 的模式修改为 `mode`（若 `filename` 为 `NULL`）。成功时返回原始 `f` 指针，失败时返回 `NULL` 且原始 `f` 被关闭。

Rust 侧实现：
- 外部接口 `freopen` 保持 `unsafe extern "C"` 的 ABI 签名
- 内部使用 RAII 模式管理临时 `f2`：若操作成功，将属性转移后 `drop` `f2`；若失败，`drop` 守卫确保资源清理
- 文件描述符复制使用 `__dup3` 系统调用（通过 syscall 模块）
- 属性转移可封装为 `FILE` 结构体的方法（`pub(crate)` 可见性），避免直接操作裸指针

### 系统算法

```
freopen(filename, mode, f):
  fl = __fmodeflags(mode)
  FLOCK(f)                           // 锁定 FILE 对象
  fflush(f)                          // 刷新当前缓冲区

  if filename.is_null():             // 无新文件: 修改当前 fd 的模式
    if fl 含 O_CLOEXEC:
      sys_fcntl(f.fd, F_SETFD, FD_CLOEXEC)
    fl 去除 O_CREAT|O_EXCL|O_CLOEXEC
    if sys_fcntl(f.fd, F_SETFL, fl) < 0:
      goto fail
  else:                              // 有新文件路径
    f2 = fopen(filename, mode)       // 打开新文件
    if f2.is_null(): goto fail
    if f2.fd == f.fd:                // 相同 fd 去重
      f2.fd = -1
    else if __dup3(f2.fd, f.fd, fl & O_CLOEXEC) < 0:
      goto fail2
    // 将 f2 的操作属性移植到 f
    f.flags  = (f.flags & F_PERM) | f2.flags
    f.read   = f2.read
    f.write  = f2.write
    f.seek   = f2.seek
    f.close  = f2.close
    fclose(f2)                       // 释放临时的 f2
                                     // 注意: 若 f2.fd == -1, close 回调不执行

  f.mode = 0
  f.locale = 0
  FUNLOCK(f)
  return f

fail2:
  fclose(f2)
fail:
  fclose(f)                          // 失败时关闭原始 f
  return NULL
```

时间复杂度 O(1)（不含系统调用及 `fopen`/`fclose` 开销）。

---

## 依赖图

```
freopen
  ├─> __fmodeflags          (see __fmodeflags.rs spec)
  ├─> FLOCK(f) / FUNLOCK(f) (宏 → 内部函数, see stdio_impl.rs)
  ├─> fflush(f)             (see fflush.rs spec)
  ├─> fopen(filename, mode) (see fopen.rs spec) [仅当 filename 非 NULL]
  ├─> __dup3                (see syscall.rs — 系统调用 dup3)
  ├─> fclose(f2)            (see fclose.rs spec)
  ├─> fclose(f)             (see fclose.rs spec) [失败路径]
  ├─> sys_fcntl             (see syscall.rs — 系统调用 fcntl)
  └─> sys_fcntl             (see syscall.rs — 系统调用 fcntl) [仅当 filename 为 NULL]
```

---

## [RELY]

- `__fmodeflags`: mode 到 open() 标志的转换（定义于 `rusl-stdio` 的 `__fmodeflags` 模块）
- `FLOCK` / `FUNLOCK`: FILE 对象锁/解锁（定义于 `rusl-internal` 的 `stdio_impl` 模块）
- `fflush`: 流缓冲区刷新（定义于 `rusl-stdio` 的 `fflush` 模块）
- `fopen`: 打开新文件（定义于 `rusl-stdio` 的 `fopen` 模块）
- `fclose`: 关闭 FILE 流（定义于 `rusl-stdio` 的 `fclose` 模块）
- `__dup3` / `sys_fcntl`: 系统调用封装（定义于 `rusl` 的 syscall 模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn freopen(
    filename: *const c_char,
    mode: *const c_char,
    f: *mut FILE,
) -> *mut FILE;
```

本模块保证对外提供 ABI 兼容的 `freopen` 符号。行为符合 musl `freopen()` 语义：成功时将已有 FILE 流重定向到新文件（或修改其模式）并返回原指针；失败时关闭原 FILE 并返回 NULL。在任何路径上都不会泄露文件描述符或 FILE 对象。
