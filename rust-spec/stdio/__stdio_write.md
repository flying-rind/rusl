# __stdio_write 函数规约

## 复杂度分级: Level 3

> musl libc 内部 FILE 默认写操作实现。作为 `f->write` 函数指针的默认值，通过 `writev` 系统调用将内部缓冲区和用户数据一并写入文件描述符，支持部分写入重试。

---

## 函数接口

```rust
use core::ffi::c_int;

extern "C" fn __stdio_write(f: *mut FILE, buf: *const u8, len: usize) -> usize;
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。作为 `f->write` 函数指针的默认值，被 `__overflow`、`fflush`、`__stdout_write` 等间接调用。

> 注意：C 原型中 `buf` 为 `const unsigned char *`，Rust 中使用 `*const u8`，二者在 ABI 层面等价。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: `*mut FILE`，非空指针，其 `fd` 为有效的文件描述符
- `buf`: `*const u8`，用户提供的数据，长度至少为 `len`
- `len`: 要写入的字节数
- `(*f).wbase` 和 `(*f).wpos` 在首次写入时为有效指针（写缓冲区中有待刷新数据）

**[Post-condition]:**

**Case 1: 完全成功写入（cnt == rem）**
- `(*f).wend = (*f).buf + (*f).buf_size`（写缓冲区全量恢复可用）
- `(*f).wpos = (*f).wbase = (*f).buf`（写缓冲区指针重置）
- 返回 `len`（用户数据全部写出）

**Case 2: 部分写入后错误（cnt < 0）**
- `(*f).wpos = (*f).wbase = (*f).wend = 0`（写缓冲区指针清零，防止使用无效数据）
- `(*f).flags |= F_ERR`（设置错误标志）
- 若 `iovcnt == 2`（内部缓冲区和用户数据都未写完）：返回 `0`（用户数据中无字节写出）
- 若 `iovcnt == 1`（内部缓冲区已写完，仅剩用户数据）：返回 `len - iov[0].iov_len`（已写出的用户数据字节数）

**Case 3: 部分写入，无错误（0 < cnt < rem）**
- 更新 `iov` 和 `rem` 以反映剩余待写数据
- 循环重试 `writev`，继续写剩余数据
- 注意：内部缓冲区（`iov[0]`）的数据优先写出

---

### 不变量

**[Invariant]:**
- 无论成功或失败，`buf` 内容不被修改（`*const u8`）
- `(*f).flags` 仅在发生错误时修改（设置 `F_ERR`）
- 写缓冲区指针仅在完全写出后重置为有效值（`buf` 和 `buf + buf_size` 之间），或在错误时清零以标记无效状态
- 错误返回时，返回值表示已从用户缓冲区 `buf` 中成功写出的字节数
- 循环内每次迭代都优先写出内部缓冲区，再写出用户数据

---

### 意图

将内部写缓冲区的剩余数据和用户数据通过一次 `writev` 系统调用一并写出。若写出不完整，循环重试直到全部写出或遇到错误。

Rust 侧实现：
- `iovec` 结构体定义为 `#[repr(C)]` 与 C 侧兼容
- `writev` 系统调用通过 `syscall!` 宏实现
- 部分写入的重试循环使用 `loop` 表达，更新 `iov` 向量的基地址和长度
- 指针操作使用 `*mut u8` 算术
- 内部缓冲区指针（`wbase`/`wpos`/`wend`）更新时注意 null 指针与有效指针的区分
- `iovcnt` 的动态缩减逻辑可用 Rust 的模式匹配清晰地表达

---

### 系统算法

```
__stdio_write(f, buf, len):
  /* 1. 构造 iovec */
  iovs: [IoVec; 2]
  iovs[0] = { f.wbase, f.wpos.offset_from(f.wbase) as usize }  // 内部缓冲区剩余数据
  iovs[1] = { buf, len }                                        // 用户数据
  rem = iovs[0].iov_len + iovs[1].iov_len
  iovcnt = 2
  iov_ptr = iovs.as_ptr()

  /* 2. 跳过空的内部缓冲区 */
  if iovs[0].iov_len == 0:
    iov_ptr = iov_ptr.add(1)
    iovcnt -= 1

  /* 3. 循环写出直到完成或错误 */
  loop:
    cnt = syscall!(SYS_writev, f.fd, iov_ptr, iovcnt)

    if cnt == rem:                              // 完全写出
      f.wend = f.buf.add(f.buf_size)
      f.wpos = f.buf
      f.wbase = f.buf
      return len

    if cnt < 0:                                 // 写出错误
      f.wpos = core::ptr::null_mut()
      f.wbase = core::ptr::null_mut()
      f.wend = core::ptr::null_mut()
      f.flags |= F_ERR
      return if iovcnt == 2 { 0 } else { len - (*iov_ptr).iov_len }

    /* 部分写出，调整 iov */
    rem -= cnt
    if cnt > (*iov_ptr).iov_len:                // 越过了第一段（内部缓冲区已写完）
      cnt -= (*iov_ptr).iov_len
      iov_ptr = iov_ptr.add(1)
      iovcnt -= 1
    (*iov_ptr).iov_base = (*iov_ptr).iov_base.add(cnt)
    (*iov_ptr).iov_len -= cnt
```

时间复杂度 O(n)（取决于部分写入重试次数，通常 1-2 次）。

---

## 依赖图

```
__stdio_write
  ├─> IoVec (平台相关 repr(C) 结构体)
  └─> syscall!(SYS_writev)   (内核)
```

---

## [RELY]

- `syscall!` 宏 — 系统调用接口（`SYS_writev`）
- `IoVec` — 散布/聚集 I/O 向量（`#[repr(C)]`，与 C 的 `struct iovec` 兼容）
- 常量: `F_ERR`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __stdio_write(f: *mut FILE, buf: *const u8, len: usize) -> usize;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为与原 C 实现完全一致：通过 `writev` 聚集写入内部缓冲区和用户数据，支持部分写入重试，正确处理写缓冲区指针和错误标志。
