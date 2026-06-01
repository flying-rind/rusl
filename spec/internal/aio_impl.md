# aio_impl.h 规约

> **源文件**: `/home/mangp/桌面/OS/musl/src/internal/aio_impl.h`
> **复杂度等级**: Level 1（简单模块 — 前置/后置条件即可描述）

---

## 依赖图

```
(无内部依赖 — 本文件仅声明全局符号)
```

本文件是 musl 的 AIO（异步 I/O）子系统内部头文件。它仅声明一个全局状态变量和两个内部辅助函数，不包含任何 `static inline` 实现或递归依赖。所有符号通过 `hidden` 可见性修饰仅对 libc 内部可见。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `<stdint.h>` 等标准头文件 | libc 公共接口 | 跳过（C 标准库） |

---

## 符号规约

---

### `__aio_fut`

```c
extern hidden volatile int __aio_fut;
```

[Visibility]: Internal — musl 内部全局变量，未通过任何 POSIX/C 标准头文件暴露。该变量用于 AIO 子系统的 fork 安全机制。

#### 功能意图 (Intent)

`__aio_fut` 是一个全局的 futex 字，用作 AIO 子系统的同步原语。当 AIO 操作正在进行时，该变量被设置为非零值，阻止在 AIO 操作完成前执行 `fork()`；`fork()` 处理程序通过该 futex 等待所有进行中的 AIO 操作完成，防止子进程继承不一致的 AIO 状态。

#### 不变量 (Invariants)

- **INV-1**: 当 `__aio_fut > 0` 时，至少有一个 AIO 操作正在进行中。
- **INV-2**: 当 `__aio_fut == 0` 时，没有进行中的 AIO 操作，`fork()` 可以安全执行。
- **INV-3**: `__aio_fut` 的值始终 >= 0，表示进行中的 AIO 操作计数。
- **INV-4**: 对 `__aio_fut` 的所有修改必须使用原子操作（通常通过 `a_inc`/`a_dec` 系列函数）。

#### 前置/后置条件

由于该变量由多个函数共享访问，其约束通过 AIO 子系统的整体不变量来保证，而非单个调用点的前置/后置条件。

---

### `__aio_close`

```c
extern hidden int __aio_close(int fd);
```

[Visibility]: Internal — musl AIO 子系统内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

关闭与 AIO 操作关联的文件描述符。与普通 `close()` 不同，此函数需要确保在关闭 fd 之前，所有针对该 fd 的异步 I/O 请求已被取消或完成，以防止竞态条件。

#### 前置条件 (Preconditions)

- **PRE-1**: `fd` 是一个有效的文件描述符，且之前已通过 AIO 子系统注册。
- **PRE-2**: 调用者必须确保 `fd` 上不再有新的 AIO 请求被提交。

#### 后置条件 (Postconditions)

- **Case 1 (成功)**: 
  - **POST-1**: 返回 0。
  - **POST-2**: `fd` 被关闭，与该 `fd` 关联的所有 AIO 控制块资源被释放。
  - **POST-3**: `__aio_fut` 中与该 `fd` 相关的计数被递减。

- **Case 2 (失败)**:
  - **POST-1**: 返回 -1，`errno` 被设置（如 `EBADF` 表示无效的 `fd`）。
  - **POST-2**: `fd` 的状态不变。

---

### `__aio_atfork`

```c
extern hidden void __aio_atfork(int);
```

[Visibility]: Internal — musl AIO 子系统内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

`fork()` 处理程序。在 `fork()` 发生前后被调用：
- 在 `fork()` 之前等待所有进行中的 AIO 操作完成（通过 `__aio_fut` futex 上阻塞）。
- 在 `fork()` 之后恢复状态。

这确保了子进程不会继承处于"进行中"状态的 AIO 请求，避免未定义行为。

#### 前置条件 (Preconditions)

- **PRE-1**: 参数指示调用阶段（0 = pre-fork, 1 = post-fork parent, 2 = post-fork child，符合 musl 内部约定）。

#### 后置条件 (Postconditions)

- **Case 1 (pre-fork, arg=0)**:
  - **POST-1**: 调用返回时，`__aio_fut == 0` 成立（所有 AIO 操作已完成）。
  - **POST-2**: 调用者可以安全执行 `fork()`。

- **Case 2 (post-fork parent, arg=1)**:
  - **POST-1**: 父进程中的 AIO 状态恢复，新的 AIO 请求可以被提交。
  - **POST-2**: `__aio_fut` 值反映父进程中进行中的 AIO 操作数量。

- **Case 3 (post-fork child, arg=2)**:
  - **POST-1**: 子进程中所有继承的 AIO 状态被清理或重置。
  - **POST-2**: `__aio_fut` 在子进程中被置为 0。

#### 不变量 (Invariants)

- **INV-1**: 在任何 `fork()` 调用点，`__aio_fut` 必须为 0（由 pre-fork 处理保证）。
- **INV-2**: 子进程中的 AIO 子系统从不保留父进程的进行中请求。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `__aio_fut` 的原子操作 | 通过 `atomic.h` 中的 `a_inc`/`a_dec`/`a_fetch_add` 等操作 | 调用方依赖 |
| futex 等待 | 通过 `__syscall(SYS_futex, ...)` / `__wait` 等内部函数 | 调用方依赖 |
| `__aio_close` 的实现 | `src/aio/aio.c` | 实现位于其他 .c 文件 |
| `__aio_atfork` 的实现 | `src/aio/aio.c` | 实现位于其他 .c 文件 |