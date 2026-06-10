# __stdio_exit 规约

## 复杂度分级: Level 1

> musl libc `__stdio_exit` 的 Rust 实现 — 程序退出时的 stdio 清理函数。遍历所有打开的 FILE 流，刷新未写入的缓冲数据，并修正文件偏移量。

---

## 函数接口

```rust
// FILE 为模块内部定义的 #[repr(C)] 结构体，对应 musl 的 struct _IO_FILE

// __stdio_exit: 程序退出时刷新所有 stdio 流的入口函数
unsafe extern "C" fn __stdio_exit();

// __stdio_exit_needed: __stdio_exit 的弱别名，供 musl 退出路径引用链判断
// Rust 侧通过 #[no_mangle] + 同一函数体实现弱别名等价
unsafe extern "C" fn __stdio_exit_needed();
```

[Visibility]:
- `__stdio_exit` — **Internal**，musl `hidden` 可见性，由 `atexit` 注册回调或 `_Exit` 路径自动调用，不直接对用户暴露。Rust 侧使用 `pub(crate)` 可见性
- `__stdio_exit_needed` — **Internal**，弱别名，供 musl 内部链接器引用链使用。若程序未链接任何 stdio 函数，`__stdio_exit_needed` 不会被定义（通过弱符号机制回退到 NULL），exit 路径将跳过 stdio 清理。Rust 侧通过 `#[no_mangle]` + 相同函数体实现
- `close_file` — **Internal**，模块内部 `fn`（非 `pub`），不对外导出
- `dummy_file` — **Internal**，模块内部 `static`，用于弱别名回退

---

## 内部函数规约

### 1. `close_file`

```rust
// 模块内部函数，不对外导出
fn close_file(f: *mut FILE);
```

[Visibility]: Internal — 模块私有函数，仅 `__stdio_exit` 内部调用

#### Intent

安全关闭/刷新单个 FILE 流。在刷新前，先检查并写出未写入的缓冲数据；再修正文件偏移量以补偿已读取但未消费的缓冲区数据。注意此函数不调用 `(*f).close`，因为进程退出时 OS 会自动回收文件描述符。

#### 前置条件

- `f`: 可能为 NULL 的 `*mut FILE` 指针

#### 后置条件

- Case 1 `f == core::ptr::null_mut()`:
  - 无操作，直接返回

- Case 2 `f != core::ptr::null_mut()`:
  - 获取 FILE 的完全锁 `FFINALLOCK(f)`
  - 若 `(*f).wpos != (*f).wbase`（写缓冲区有待刷新数据）: 调用 `(*f).write(f, core::ptr::null(), 0)` 刷新写缓冲区
  - 若 `(*f).rpos != (*f).rend`（读缓冲区已被读取但未消费完）: 调用 `(*f).seek(f, (*f).rpos - (*f).rend, SEEK_CUR)` 回退文件位置以补偿多读的数据
  - 不调用 `(*f).close`（退出时 OS 自动回收）

#### 系统算法

```
close_file(f: *mut FILE):
  1. if f == core::ptr::null_mut(): return
  2. FFINALLOCK(f)                           // 获取 FILE 完全锁
  3. if (*f).wpos != (*f).wbase:             // 有待刷新写数据
       let write_fn = (*f).write.unwrap()     // 安全解包 write 函数指针
       write_fn(f, core::ptr::null(), 0)      // 刷新写缓冲区
  4. if (*f).rpos != (*f).rend:              // 有未消费读数据
       let offset = (*f).rpos - (*f).rend     // 计算回退偏移量
       let seek_fn = (*f).seek.unwrap()       // 安全解包 seek 函数指针
       seek_fn(f, offset, SEEK_CUR)           // 回退文件位置
  5. // 不调用 f->close — 退出时 OS 自动回收 fd
```

时间复杂度 O(1)（单流操作）。

Rust 侧实现要点：
- 函数为模块私有 `fn`（非 `extern "C"`），可用安全 Rust 重构内部逻辑
- `FFINALLOCK` 在 Rust 侧可封装为 `RAII` 锁守卫，自动释放锁
- 函数指针字段 `write`/`seek` 在 Rust 侧为 `Option<unsafe extern "C" fn(...)>`，调用前需解包
- NULL 检查在函数入口处完成，保证后续指针解引用安全

---

## 对外函数规约

### 2. `__stdio_exit`

```rust
unsafe extern "C" fn __stdio_exit();
```

[Visibility]: Internal — `pub(crate)`，musl `hidden` 可见性，供 `atexit` 回调或 `_Exit` 路径调用

#### Intent

程序退出时清理所有 stdio 流的入口函数。由 musl 的 exit 路径（`exit()` / `_Exit()`）自动调用，确保所有缓冲数据被刷新到文件系统。

**两层遍历**：
1. 先遍历全局打开文件链表（`ofl_head`）中的显式打开流（如 `fopen` 创建的用户流）
2. 再处理 `__stdin_used`、`__stdout_used`、`__stderr_used` 三个标准流（无论是否被显式引用）

#### 前置条件

- 程序即将退出（由 `atexit` 注册的回调或 `_Exit` 路径触发）
- `__ofl_lock()` 返回的全局文件链表可能包含已打开的 FILE 对象，也可能为空
- `__stdin_used`、`__stdout_used`、`__stderr_used` 可能指向有效 FILE 对象，也可能为 NULL（如果链接时未引用任何 stdio 函数，它们通过弱别名机制指向零值）

#### 后置条件

- 全局文件链表中的所有 FILE 流已被刷新（写缓冲区数据已写出）
- `__stdin_used`、`__stdout_used`、`__stderr_used` 指向的流已被刷新（若非 NULL）
- 所有流的文件偏移量已调整以反映实际读/写位置
- 不关闭任何文件描述符（OS 在进程终止时自动回收）

#### 系统算法

```
__stdio_exit():
  1. // 第一层：遍历全局打开文件链表
     let head_ptr = __ofl_lock()              // 获取锁和链表头
     f = *head_ptr                            // 当前节点
     while f != core::ptr::null_mut():
         close_file(f)                        // 刷新当前 FILE
         f = (*f).next                        // 移至下一个节点
     // 注意：锁在遍历期间始终持有，确保链表不被并发修改
     // 锁不显式释放——因为进程即将退出，由 OS 回收

  2. // 第二层：处理三个标准流
     close_file(__stdin_used)                 // 刷新 stdin（若非 NULL）
     close_file(__stdout_used)                // 刷新 stdout（若非 NULL）
     close_file(__stderr_used)                // 刷新 stderr（若非 NULL）
```

时间复杂度 O(N)，N 为全局链表中的 FILE 对象数量。

---

### 3. `__stdio_exit_needed` (weak_alias)

```rust
// __stdio_exit 的弱别名，同一函数体
// Rust 侧使用 #[no_mangle] 导出同名符号
unsafe extern "C" fn __stdio_exit_needed();
```

[Visibility]: Internal — 弱别名，供 musl 内部链接器引用链使用

#### Intent

`__stdio_exit` 的弱别名。musl 的退出路径通过 `__stdio_exit_needed` 符号引用来判断是否需要调用 stdio 清理。若程序中未链接任何 stdio 函数，`__stdio_exit_needed` 不会被定义（通过弱符号机制），exit 路径将跳过 stdio 清理逻辑。

Rust 侧实现：使用 `#[no_mangle]` 导出相同函数体的符号。若 Rust 链接器支持弱符号（通过 `#[link_section]` 或链接脚本），可实现真正的弱别名；否则通过复制函数体实现等价语义。

---

### 4. 弱别名哨兵变量

```rust
// 内部哑变量，用于弱别名回退
static mut dummy_file: *mut FILE = core::ptr::null_mut();

// 以下三个符号通过弱别名机制定义：
// weak_alias(dummy_file, __stdin_used);
// weak_alias(dummy_file, __stdout_used);
// weak_alias(dummy_file, __stderr_used);
```

[Visibility]: Internal — 弱别名定义，仅当链接时未引用 stdin/stdout/stderr 相关模块的强定义时生效

#### Intent

若程序未链接任何引用 stdin/stdout/stderr 的代码（即 `__stdin_used`、`__stdout_used`、`__stderr_used` 的强定义未出现在链接中），则通过弱别名将它们替换为 NULL（`dummy_file` 的值）。这样 `__stdio_exit` 在访问它们时，`close_file(NULL)` 会安全地无操作返回。

Rust 侧实现：
- 若 Rust 链接器支持弱符号（通过 `#[link_section(".gnu.linkonce")]` 或类似机制），可实现真正的弱别名
- 若不支持，可直接依赖 `__stdin_used`、`__stdout_used`、`__stderr_used` 模块提供的强定义（因为它们始终存在于 rusl 中），省略 `dummy_file` 弱别名机制
- 无论哪种方式，`close_file` 函数入口处的 NULL 检查确保安全

---

## 不变量

**[Invariant]:**
- **弱别名兜底**：若 stdin/stdout/stderr 未被实际引用，`__std*_used` 变量为 NULL，`close_file` 会在函数入口处安全跳过
- **缓冲区一致性**：退出时，每个流的写缓冲区必须刷新（`wpos` 回到 `wbase`），读缓冲区的文件偏移量必须修正（通过 `seek` 回退未消费数据）
- **退出安全**：`__stdio_exit` 可以多次调用而不会崩溃（第二次调用时链表可能为空或含已处理流）
- **双层保护**：先处理全局链表中的用户流，再处理三个标准流，确保所有打开流都被刷新

---

## 意图

程序退出时确保所有缓冲数据不丢失。`__stdio_exit` 是 stdio 生命周期的终点，与 `__ofl_add`（生命周期起点）形成对称。

Rust 侧实现要点：
- `close_file` 为模块私有 `fn`，可用安全 Rust 重构内部操作（RAII 锁守卫、Option 解包等）
- `__stdio_exit` 为 `unsafe extern "C" fn`，保持与 C 侧 ABI 兼容
- `__stdio_exit_needed` 为 `#[no_mangle]` 导出的等价函数体，用于链接器引用链
- 弱别名哨兵变量在 Rust 侧可简化处理：由于 rusl 始终包含三个标准流的强定义，`dummy_file` 弱别名机制可按需实现
- 若 Rust 链接器不支持弱符号，可使用条件编译或链接脚本实现等价语义

---

## 依赖图

```
__stdio_exit (Internal)
  ├── __ofl_lock          (see ofl spec)      ──> *mut *mut FILE, 获取链表头
  ├── close_file (static, 模块内部)
  │     ├── FFINALLOCK    (锁宏)               ──> __lockfile (see __lockfile spec)
  │     ├── f->write      (FILE 函数指针)       ──> __stdio_write / __stdout_write
  │     ├── f->seek       (FILE 函数指针)       ──> __stdio_seek
  │     └── (直接访问 f->wpos, f->wbase, f->rpos, f->rend)
  └── __stdin_used / __stdout_used / __stderr_used (弱别名哨兵)
```

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `SEEK_CUR` | 1 | `stdio_impl` 模块 | 相对于当前位置的文件定位 |

---

## [RELY]

- `__ofl_lock()` — 获取全局打开文件链表头指针（见 `ofl` spec）
- `FFINALLOCK` — FILE 完全锁宏，Rust 侧封装为 `__lockfile` 调用（见 `__lockfile` spec）
- `FILE.write` — 流写函数指针（见 `__stdio_write` / `__stdout_write` spec）
- `FILE.seek` — 流定位函数指针（见 `__stdio_seek` spec）
- `__stdin_used` / `__stdout_used` / `__stderr_used` — 标准流哨兵变量（见 `stdin` / `stdout` / `stderr` spec）
- `FILE` 结构体定义 — `wpos`, `wbase`, `rpos`, `rend`, `next` 字段布局（见 `stdio_impl` 模块）
- `SEEK_CUR` 常量 — 值为 1

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn __stdio_exit();
unsafe extern "C" fn __stdio_exit_needed();
```

本模块保证对外提供上述 ABI 兼容的函数符号：
- `__stdio_exit`: 程序退出时遍历全局 FILE 链表和三个标准流，刷新所有缓冲数据，修正文件偏移量。该符号为 Internal 可见性，不直接对用户暴露，由 exit 路径自动调用
- `__stdio_exit_needed`: `__stdio_exit` 的弱别名，用于链接器引用链判断。若未链接 stdio，通过弱符号机制跳过 stdio 清理

全局不变量保证：退出后所有未写入缓冲数据已刷新，所有流的文件位置已修正，不关闭任何文件描述符（OS 回收）。
