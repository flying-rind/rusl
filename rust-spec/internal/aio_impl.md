# aio_impl 模块规约 (Rust)

> **源 C spec**: `/home/mangp/桌面/OS/musl/src/internal/spec/aio_impl.md`
> **复杂度等级**: Level 1（简单模块 — 前置/后置条件即可描述）

---

## 依赖图

```
(内部) crate::sys::futex ──> aio_impl 模块
(内部) core::sync::atomic ──> aio_impl 模块
```

本模块是 rusl 的 AIO（异步 I/O）子系统内部模块。在 Rust 实现中，原 C 的全局 `volatile int __aio_fut` 替换为 `AtomicI32`，利用 Rust 的类型安全保证原子访问。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::sync::atomic::AtomicI32` | Rust core 库 | 替换 C 的 `volatile int` + `a_inc`/`a_dec` |
| `crate::sys::futex` | rusl 内部 futex 封装 | 提供 `futex_wait`/`futex_wake` 系统调用封装 |

---

## 符号规约

---

### `AIO_FUT`

```rust
// Rust 声明
static AIO_FUT: AtomicI32 = AtomicI32::new(0);
```

[Visibility]: Internal — rusl AIO 子系统内部全局状态，未通过任何公共 API 暴露。

#### 功能意图 (Intent)

`AIO_FUT` 是一个全局原子计数器，用作 AIO 子系统的同步原语。当 AIO 操作正在进行时，该计数器为非零值，阻止在 AIO 操作完成前执行 `fork()`；`fork()` 处理程序通过 futex 等待所有进行中的 AIO 操作完成，防止子进程继承不一致的 AIO 状态。

#### 不变量 (Invariants)

- **INV-1**: 当 `AIO_FUT.load(Ordering::SeqCst) > 0` 时，至少有一个 AIO 操作正在进行中。
- **INV-2**: 当 `AIO_FUT.load(Ordering::SeqCst) == 0` 时，没有进行中的 AIO 操作，`fork()` 可以安全执行。
- **INV-3**: `AIO_FUT` 的值始终 >= 0，表示进行中的 AIO 操作计数。
- **INV-4**: 对 `AIO_FUT` 的所有修改必须使用 `AtomicI32` 的原子操作方法（`fetch_add`/`fetch_sub`）。

#### 与 C 实现的差异

C 实现使用 `volatile int` + 宏 `a_inc`/`a_dec`。Rust 实现使用 `AtomicI32`，提供更强的类型安全和内存顺序保证。`AtomicI32::fetch_add(1, Ordering::SeqCst)` 等价于 C 的 `a_inc`。

---

### `__aio_close`

```rust
// Rust 声明 — 内部函数，不对外导出
pub(crate) extern "C" fn __aio_close(fd: c_int) -> c_int;
```

- 由于 musl libc 其他模块（如 `__stdio_close`）可能通过符号名直接调用 `__aio_close`，需保持 `extern "C"` ABI 和符号名不变。
- 在纯 rusl 环境中，若不存在跨语言调用，可简化为普通 `pub(crate) fn aio_close(fd: RawFd) -> Result<(), Error>`。

[Visibility]: Internal — rusl AIO 子系统内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

关闭与 AIO 操作关联的文件描述符。与普通 `close()` 不同，此函数需要确保在关闭 fd 之前，所有针对该 fd 的异步 I/O 请求已被取消或完成，以防止竞态条件。

#### 前置条件 (Preconditions)

- **PRE-1**: `fd` 是一个有效的文件描述符，且之前已通过 AIO 子系统注册。
- **PRE-2**: 调用者必须确保 `fd` 上不再有新的 AIO 请求被提交。

#### 后置条件 (Postconditions)

- **Case 1 (成功)**:
  - **POST-1**: 返回 0。
  - **POST-2**: `fd` 被关闭，与该 `fd` 关联的所有 AIO 控制块资源被释放。
  - **POST-3**: `AIO_FUT` 中与该 `fd` 相关的计数被递减。

- **Case 2 (失败)**:
  - **POST-1**: 返回 -1，`errno` 被设置（如 `EBADF` 表示无效的 `fd`）。
  - **POST-2**: `fd` 的状态不变。

#### Rust 实现建议

在 rusl 内部实现中，建议将 AIO 控制块封装为结构体 `AioControlBlock`，使用 `Arc` 或内部引用计数管理生命周期。`aio_close` 通过递减引用计数并在计数归零时清理资源来实现。

---

### `__aio_atfork`

```rust
// Rust 声明 — 内部函数，不对外导出
pub(crate) extern "C" fn __aio_atfork(arg: c_int);
```

[Visibility]: Internal — rusl AIO 子系统内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

`fork()` 处理程序。在 `fork()` 发生前后被调用：
- 在 `fork()` 之前等待所有进行中的 AIO 操作完成（通过 `AIO_FUT` futex 上阻塞）。
- 在 `fork()` 之后恢复状态。

这确保了子进程不会继承处于"进行中"状态的 AIO 请求，避免未定义行为。

#### 前置条件 (Preconditions)

- **PRE-1**: 参数指示调用阶段（0 = pre-fork, 1 = post-fork parent, 2 = post-fork child，符合 musl 内部约定）。

#### 后置条件 (Postconditions)

- **Case 1 (pre-fork, arg=0)**:
  - **POST-1**: 调用返回时，`AIO_FUT.load(Ordering::SeqCst) == 0` 成立（所有 AIO 操作已完成）。
  - **POST-2**: 调用者可以安全执行 `fork()`。

- **Case 2 (post-fork parent, arg=1)**:
  - **POST-1**: 父进程中的 AIO 状态恢复，新的 AIO 请求可以被提交。
  - **POST-2**: `AIO_FUT` 值反映父进程中进行中的 AIO 操作数量。

- **Case 3 (post-fork child, arg=2)**:
  - **POST-1**: 子进程中所有继承的 AIO 状态被清理或重置。
  - **POST-2**: `AIO_FUT` 在子进程中被置为 0。

#### 不变量 (Invariants)

- **INV-1**: 在任何 `fork()` 调用点，`AIO_FUT` 必须为 0（由 pre-fork 处理保证）。
- **INV-2**: 子进程中的 AIO 子系统从不保留父进程的进行中请求。

#### Rust 实现建议

在 rusl 中，`AIO_FUT` 为 `AtomicI32`。pre-fork 阶段使用 futex 等待循环：
```rust
while AIO_FUT.load(Ordering::SeqCst) > 0 {
    futex_wait(&AIO_FUT, ...);
}
```
post-fork 阶段（子进程）直接将 `AIO_FUT` 置为 0。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `AIO_FUT` | 本模块 | 全局原子状态 |
| futex 系统调用 | `crate::sys::futex` | 等待/唤醒原语 |
| `__aio_close` 的实现 | `src/aio/mod.rs` | 实现位于 aio 模块 |
| `__aio_atfork` 的实现 | `src/aio/mod.rs` | 实现位于 aio 模块 |