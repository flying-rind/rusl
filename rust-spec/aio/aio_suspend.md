# aio_suspend 规约

> 对应 C 源文件: `musl-1.2.6/src/aio/aio_suspend.c`
> 对应 Rust 模块: `rusl-aio/src/aio_suspend.rs`
> 对外符号: `aio_suspend`

---

## 复杂度分级

**[Complexity]: Level 2** — 包含复杂的 futex 同步逻辑与多路异步等待算法。

---

## 函数接口

```rust
use core::ffi::c_int;

// aiocb 和 timespec 为 repr(C) 结构体，由其他 crate 定义
// use rusl_aio::aiocb;
// use rusl_time::timespec;

/// 挂起调用线程，直到至少一个异步 I/O 操作完成、超时到期、或被信号中断。
///
/// 此函数是 POSIX 线程取消点。
#[no_mangle]
pub extern "C" fn aio_suspend(
    cbs: *const *const aiocb,
    cnt: c_int,
    ts: *const timespec,
) -> c_int;
```

**[Visibility]: User** — 声明于 `<aio.h>`，POSIX 标准异步 I/O 接口，用户可直接调用。

---

## 内部依赖追踪

### 依赖的来自本模块其他文件的符号

| 符号 | 类型 | 定义位置 | 说明 |
|------|------|----------|------|
| `__aio_fut` | 全局变量 | `aio.rs` | 多 aiocb 挂起时的全局 futex 等待字，`cleanup()` 中置零并唤醒。Rust 内部以 `AtomicI32` 实现，通过 `core::sync::atomic::Ordering::AcqRel` 操作。 |
| `aio_error` | 函数 | `aio.rs` | 查询 AIO 操作状态，返回 `cb.__err & 0x7fffffff`。Rust 签名为 `pub extern "C" fn aio_error(cb: *const aiocb) -> c_int;` |

### 使用的结构体字段

| 字段 | 所属结构体 | 用途 |
|------|-----------|------|
| `__err` | `struct aiocb` (Rust: `aiocb.__err: c_int`) | 单 aiocb 时的 futex 等待字，高位置位表示有等待者 |

### 依赖的外部模块接口

| 接口 | Rust 定义位置 | 用途 |
|------|-------------|------|
| `__pthread_self` | `rusl_pthread::__pthread_self` | 获取当前线程指针，用于获取线程 tid |
| `__timedwait_cp` | `rusl_internal::__timedwait_cp` | 带超时和取消点的 futex 等待 |
| `pthread_testcancel` | `rusl_pthread::pthread_testcancel` | 线程取消点检查 |
| `clock_gettime` | `rusl_time::clock_gettime` | 获取单调时钟当前值 |
| `CLOCK_MONOTONIC` | `rusl_time::CLOCK_MONOTONIC` | 单调时钟 ID |
| `AtomicI32` (替代 `a_cas`, `a_swap`) | `core::sync::atomic::AtomicI32` | 原子 CAS 和 swap 操作 |

---

## 规约

### Intent（意图）

挂起调用线程，直到 `cbs` 数组中至少一个异步 I/O 操作完成、调用被信号中断、或超时到期。函数为 POSIX 线程取消点，也是 musl AIO 框架中用于跨多个 aiocb 同步的 futex 等待机制的核心入口。

使用 futex 而非 pthread 条件变量实现等待，以保证 `aio_cancel` / `close` 调用方的异步信号安全性（`aio_cancel` 由 `close` 调用，`close` 必须为 async-signal-safe）。

---

### Pre-condition（前置条件）

- `cbs`: 指针数组，长度为 `cnt`。数组条目可为 NULL（被忽略）；非 NULL 条目指向已成功提交（通过 `aio_read`/`aio_write`/`lio_listio`）且尚未调用 `aio_return` 的 `aiocb`。
- `cnt`: 数组长度。若为 0，则 `cbs` 被完全忽略（无操作可等待，函数等价于仅等待超时或信号）。
- `ts`: 指向相对超时时间的指针。NULL 表示无限等待；非 NULL 时 `tv_sec` 和 `tv_nsec` 构成一个有效的 `struct timespec`（`tv_nsec` 范围 [0, 999999999]）。
- 调用线程已阻塞所有信号（非强制，但为正确性建议，见 Invariant 节）。
- `errno` 的当前值不参与任何语义；函数总是覆写 `errno` 于错误路径。

---

### Post-condition（后置条件）

#### Case 1: 至少一个 AIO 操作已完成（成功返回）

- 返回值: `0`
- `errno`: 未修改（保持调用前的值）
- 状态: `cbs` 中至少存在一个非 NULL 条目 `i`，使得 `aio_error(cbs[i]) != EINPROGRESS`。该操作的结果可通过 `aio_return(cbs[i])` 获取。
- 副作用:
  - 若仅有单个非 NULL aiocb，`cb.__err` 的高位可能已被原子置位（`EINPROGRESS | 0x80000000`），表示有等待者存在过。
  - 若有多个非 NULL aiocb，`__aio_fut` 的值被 I/O 完成路径以原子 swap 置零并唤醒。
  - 若 `cnt` 为 0 或无任何非 NULL 条目，函数在 futex 等待返回后无条件返回 0。

#### Case 2: 所有操作仍在进行中且超时到期

- 返回值: `-1`
- `errno`: `EAGAIN`
- 状态: `cbs` 中所有非 NULL 条目仍处于 `EINPROGRESS` 状态。

#### Case 3: 被信号中断

- 返回值: `-1`
- `errno`: `EINTR`
- 状态: I/O 操作的完成状态不变（可能已完成也可能未完成）。

#### Case 4: 被线程取消

- 返回值: `-1`
- `errno`: `ECANCELED`
- 行为: 函数为线程取消点；清理 handler 在 `__timedwait_cp` 中被执行，线程随后终止。调用方实际上不会观察到此返回值。

#### Case 5: 参数无效

- 前置条件: `cnt < 0`
- 返回值: `-1`
- `errno`: `EINVAL`
- 状态: 无 I/O 操作被查询，无等待发生。

---

### System Algorithm（系统算法）

Rust 实现中，系统算法与 C 版本逻辑一致，但内部依赖的原子操作使用 `core::sync::atomic` 类型的 safe 接口，`unsafe` 仅用于必要的 FFI 调用和裸指针解引用。

```
1. 线程取消点检查: 调用 pthread_testcancel()
   注意: 此调用从此函数的 Rust 代码路径出发，最终通过 FFI 调用到 rusl_pthread 的实现

2. 参数验证:
   - 若 cnt < 0: 设置 errno = EINVAL, 返回 -1

3. 第一趟快速扫描 (快速路径):
   - nzcnt = 0 (非 NULL aiocb 计数)
   - cb: *const aiocb = null_mut() (记录最后一个非 NULL aiocb)
   - 遍历 i = 0..cnt:
     - 在 unsafe 块内读取 cbs.add(i):
       - 若 *cbs.add(i) 非 NULL:
         - 在 unsafe 块内调用 aio_error(*cbs.add(i)):
           - 若返回值 != EINPROGRESS → 立即返回 0
         - nzcnt += 1
         - cb = *cbs.add(i)

4. 超时时间计算 (若 ts 非 NULL):
   - 在 unsafe 块内调用 clock_gettime(CLOCK_MONOTONIC, &mut at)
   - at = at + ts (处理 tv_nsec 进位到 tv_sec)

5. 主等待循环:
   LOOP:
     a. 再次检查: 遍历所有非 NULL 条目
        - 若任一 aio_error(cbs[i]) != EINPROGRESS → 返回 0

     b. 根据 nzcnt 选择 futex 等待策略:
        - nzcnt == 0:
          在 Rust 中声明局部变量 dummy_fut: AtomicI32 = AtomicI32::new(0)
          pfut = &dummy_fut as *const AtomicI32 as *const c_int
          expect = 0

        - nzcnt == 1:
          在 unsafe 块内获取 &(*cb).__err 作为 pfut 地址
          expect = EINPROGRESS | 0x80000000
          使用 AtomicI32 的 compare_exchange 执行原子 CAS:
            (*pfut).compare_exchange(EINPROGRESS, expect, AcqRel, Acquire)
          (在 Rust 中需确保 __err 字段为 AtomicI32 类型，
           或通过裸指针使用 core::intrinsics::atomic_cxchg)

        - nzcnt >= 2:
          pfut = &__aio_fut as *const AtomicI32 as *const c_int
          若 tid 为 0:
            tid = __pthread_self().tid (在 unsafe 块中通过 FFI 调用)
          // 使用 __aio_fut (AtomicI32) 的 compare_exchange 执行 CAS
          // expect = __aio_fut.compare_exchange(0, tid, AcqRel, Acquire)
          若 expect == 0:
            expect = tid  // 本线程是第一个等待者

          // 注册后再次检查，防止在注册期间有 I/O 完成
          遍历所有非 NULL 条目:
            若 aio_error(cbs[i]) != EINPROGRESS → 返回 0

     c. futex 等待 (线程取消点):
        在 unsafe 块内调用:
          ret = __timedwait_cp(pfut, expect, CLOCK_MONOTONIC,
                               ts ? &at : null(), 1)
           // 最后一个参数 1 表示 PRIVATE futex

     d. 处理等待返回:
        - ret == ETIMEDOUT: errno = EAGAIN, 返回 -1
        - ret == ECANCELED 或 EINTR: errno = ret, 返回 -1
        - 否则: 继续 LOOP (被虚假唤醒或 I/O 已完成)
```

---

### Invariant（不变量）

1. **futex 地址有效性**: 在调用 `__timedwait_cp` 时，`pfut` 指向的地址始终有效：
   - `nzcnt == 0`: 指向栈上局部变量 `dummy_fut`（始终有效且永不变值）。在 Rust 中确保该局部变量通过 `Pin` 或安全的借用规则保证地址在等待期间有效。
   - `nzcnt == 1`: 指向 `cb.__err`。`cb` 所指向的 `aiocb` 在 `aio_return` 被调用前必须保持有效（POSIX 要求），因此地址在等待期间有效。
   - `nzcnt >= 2`: 指向全局变量 `__aio_fut`（始终有效）。

2. **无竞态遗漏通知**: 在注册等待者身份后（对 `__aio_fut` 执行 CAS 后），函数重新扫描 `cbs` 数组。这保证：若某个 I/O 操作恰好在注册等待者与进入 futex 等待之间完成，其 `cleanup` 路径中的原子 swap（`__aio_fut.swap(0, AcqRel)`）和 `__wake` 要么在重新扫描前发生（被重新扫描检测到并返回 0），要么在 `__timedwait_cp` 进入等待后发生（被 futex 唤醒）。

3. **EINPROGRESS 编码约定**: `__err` 字段的低 31 位存储实际错误码，高位（bit 31）用作 "有等待者" 标志。`aio_error()` 通过 `& 0x7fffffff` 掩码读取，因此等待者的存在对 `aio_error()` 结果透明。

---

## Unsafe 使用边界

在 Rust 实现中，`unsafe` 块严格限定于以下操作：

1. **裸指针解引用**: `*cbs.add(i)` 读取 aiocb 指针
2. **FFI 调用**: `aio_error()`, `clock_gettime()`, `__timedwait_cp()`, `pthread_testcancel()`, `__pthread_self()`
3. **原子操作地址获取**: 将 `&__aio_fut` (AtomicI32) 的地址转换为 `*const c_int` 以传递给 futex 等待函数
4. **`errno` 设置**: 通过 rusl-errno 的 safe 封装函数或直接写入（视 rusl-errno 的 API 设计而定）

函数签名本身为 safe（`pub extern "C" fn`），所有 unsafe 操作封装在函数体内部。

---

## 与 C 实现的差异说明

| 方面 | C 实现 (musl) | Rust 实现 (rusl) |
|------|--------------|-----------------|
| 原子操作 | `a_cas()`, `a_swap()` 宏（依赖 `internal/atomic.h`） | `core::sync::atomic::AtomicI32::compare_exchange()` / `swap()` |
| 取消点实现 | `pthread_cleanup_push`/`pthread_cleanup_pop` 宏 | Rust scope guard 模式或内联展开，等效于 C 语义 |
| 内存安全 | 依赖开发者手动管理所有指针 | 编译器强制执行借用规则，减少悬垂指针风险 |
| 错误码设置 | 直接写入全局 `errno` | 通过 `rusl_errno::set_errno()` safe 封装函数 |
| futex 地址 | 直接将 `int*` 传递给 `__timedwait_cp` | 将 `AtomicI32` 引用转换为 `*const c_int`（保证地址与 C 一致） |

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  aiocb — rusl-aio crate 定义的 repr(C) 结构体
  timespec — rusl-time crate 定义的 repr(C) 结构体
  aio_error — rusl-aio crate 导出的 extern "C" 函数
  __aio_fut — rusl-aio crate 内部的全局 AtomicI32（对外暴露为可寻址 c_int）
  __pthread_self — rusl-pthread crate 内部接口
  __timedwait_cp — rusl-internal crate 提供的 futex 等待函数
  pthread_testcancel — rusl-pthread crate 导出的 extern "C" 函数
  clock_gettime — rusl-time crate 导出的 extern "C" 函数
Predefined Macros/Crates:
  core::ffi — c_int, c_void
  core::sync::atomic — AtomicI32, Ordering (AcqRel, Acquire, Release, SeqCst)
  rusl_errno — crate 内部错误码管理 (set_errno, EAGAIN, EINTR, ECANCELED, EINVAL, EINPROGRESS, ETIMEDOUT)
  rusl_aio — 本 crate 内部引用的其他模块符号

[GUARANTEE]
Exported Interface:
  aio_suspend: pub extern "C" fn (cbs: *const *const aiocb, cnt: c_int, ts: *const timespec) -> c_int
    - ABI 与 C 版本完全兼容：参数顺序、类型布局、返回值布局、调用约定一致
    - 函数签名为 safe（外部 C 代码可透明调用）
    - unsafe 仅存在于函数体内部的 FFI 调用和裸指针操作
Internal Interface:
  所有内部辅助函数和局部变量均为模块私有（pub(crate) 或更低可见性）
  原子操作使用 core::sync::atomic 的 safe 接口实现，不引入新的 unsafe 路径
