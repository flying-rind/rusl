# aio_suspend.c 规约

> 对应源文件: `musl-1.2.6/src/aio/aio_suspend.c`
> 对外符号: `aio_suspend`

---

## 内部依赖追踪

### 依赖的来自本模块其他文件的符号

| 符号 | 类型 | 定义位置 | 说明 |
|------|------|----------|------|
| `__aio_fut` | 全局变量 | `aio.c:77` | 多 aiocb 挂起时的全局 futex 等待字，`cleanup()` 中置零并唤醒 |
| `aio_error()` | 函数 | `aio.c:354` | 查询 AIO 操作状态，返回 `cb->__err & 0x7fffffff` |

### 使用的结构体字段

| 字段 | 所属结构体 | 用途 |
|------|-----------|------|
| `cb->__err` | `struct aiocb` | 单 aiocb 时的 futex 等待字，高位置位表示有等待者 |

---

## 符号规约

### `aio_suspend`

```c
int aio_suspend(const struct aiocb *const cbs[], int cnt, const struct timespec *ts);
```

**[Visibility]: User** — 声明于 `<aio.h>`，POSIX 标准异步 I/O 接口，用户可直接调用。

**[Complexity]: Level 2** — 包含复杂的 futex 同步逻辑与多路异步等待算法。

---

#### Intent（意图）

挂起调用线程，直到 `cbs` 数组中至少一个异步 I/O 操作完成、调用被信号中断、或超时到期。函数为 POSIX 线程取消点，也是 musl AIO 框架中用于跨多个 aiocb 同步的 futex 等待机制的核心入口。

使用 futex 而非 pthread 条件变量实现等待，以保证 `aio_cancel` / `close` 调用方的异步信号安全性（`aio_cancel` 由 `close` 调用，`close` 必须为 async-signal-safe）。

---

#### Pre-condition（前置条件）

- `cbs`: 指针数组，长度为 `cnt`。数组条目可为 NULL（被忽略）；非 NULL 条目指向已成功提交（通过 `aio_read`/`aio_write`/`lio_listio`）且尚未调用 `aio_return` 的 `struct aiocb`。
- `cnt`: 数组长度。若为 0，则 `cbs` 被完全忽略（无操作可等待，函数等价于仅等待超时或信号）。
- `ts`: 指向相对超时时间的指针。NULL 表示无限等待；非 NULL 时 `tv_sec` 和 `tv_nsec` 构成一个有效的 `struct timespec`（`tv_nsec` 范围 [0, 999999999]）。
- 调用线程已阻塞所有信号（非强制，但为正确性建议，见 Invariant 节）。
- `errno` 的当前值不参与任何语义；函数总是覆写 `errno` 于错误路径。

---

#### Post-condition（后置条件）

##### Case 1: 至少一个 AIO 操作已完成（成功返回）

- 返回值: `0`
- `errno`: 未修改（保持调用前的值）
- 状态: `cbs` 中至少存在一个非 NULL 条目 `i`，使得 `aio_error(cbs[i]) != EINPROGRESS`。该操作的结果可通过 `aio_return(cbs[i])` 获取。
- 副作用:
  - 若仅有单个非 NULL aiocb，`cb->__err` 的高位可能已被原子置位（`EINPROGRESS | 0x80000000`），表示有等待者存在过。
  - 若有多个非 NULL aiocb，`__aio_fut` 的值被 I/O 完成路径以 `a_swap` 置零并唤醒。
  - 若 `cnt` 为 0 或无任何非 NULL 条目，函数在 futex 等待返回后无条件返回 0。

##### Case 2: 所有操作仍在进行中且超时到期

- 返回值: `-1`
- `errno`: `EAGAIN`
- 状态: `cbs` 中所有非 NULL 条目仍处于 `EINPROGRESS` 状态。

##### Case 3: 被信号中断

- 返回值: `-1`
- `errno`: `EINTR`
- 状态: I/O 操作的完成状态不变（可能已完成也可能未完成）。

##### Case 4: 被线程取消

- 返回值: `-1`
- `errno`: `ECANCELED`
- 行为: 函数为线程取消点；清理 handler 在 `__timedwait_cp` 中被执行，线程随后终止。调用方实际上不会观察到此返回值。

##### Case 5: 参数无效

- 前置条件: `cnt < 0`
- 返回值: `-1`
- `errno`: `EINVAL`
- 状态: 无 I/O 操作被查询，无等待发生。

---

#### System Algorithm（系统算法）

```
1. 线程取消点检查: 调用 pthread_testcancel()
2. 参数验证:
   - 若 cnt < 0: 设置 errno = EINVAL, 返回 -1
3. 第一趟快速扫描 (快速路径):
   - nzcnt = 0 (非 NULL aiocb 计数)
   - cb = 0 (记录最后一个非 NULL aiocb)
   - 遍历 i = 0..cnt-1:
     - 若 cbs[i] 非 NULL:
       - 若 aio_error(cbs[i]) != EINPROGRESS → 操作已完成，立即返回 0
       - nzcnt++
       - cb = cbs[i]
4. 超时时间计算 (若 ts 非 NULL):
   - clock_gettime(CLOCK_MONOTONIC, &at)
   - at = at + ts (处理 tv_nsec 进位到 tv_sec)
5. 主等待循环:
   LOOP:
     a. 再次检查: 遍历所有非 NULL 条目
        - 若任一 aio_error(cbs[i]) != EINPROGRESS → 返回 0
     b. 根据 nzcnt 选择 futex 等待策略:
        - nzcnt == 0:
          pfut = &dummy_fut (局部变量，永不变值)
          expect = 0
        - nzcnt == 1:
          pfut = &cb->__err
          expect = EINPROGRESS | 0x80000000
          a_cas(pfut, EINPROGRESS, expect)  // 原子标记"有等待者"
        - nzcnt >= 2:
          pfut = &__aio_fut
          若 tid 为 0:
            tid = __pthread_self()->tid
          expect = a_cas(pfut, 0, tid)  // 尝试注册本线程为等待者
          若 expect == 0:
            expect = tid  // 本线程是第一个等待者
          // 注册后再次检查，防止在注册期间有 I/O 完成
          遍历所有非 NULL 条目:
            若 aio_error(cbs[i]) != EINPROGRESS → 返回 0
     c. futex 等待 (线程取消点):
        ret = __timedwait_cp(pfut, expect, CLOCK_MONOTONIC,
                             ts ? &at : NULL, 1)
        // 最后一个参数 1 表示 PRIVATE futex
     d. 处理等待返回:
        - ret == ETIMEDOUT: errno = EAGAIN, 返回 -1
        - ret == ECANCELED 或 EINTR: errno = ret, 返回 -1
        - 否则: 继续 LOOP (被虚假唤醒或 I/O 已完成)
```

---

#### Invariant（不变量）

1. **futex 地址有效性**: 在调用 `__timedwait_cp` 时，`pfut` 指向的地址始终有效：
   - `nzcnt == 0`: 指向栈上局部变量 `dummy_fut`（始终有效且永不变值）
   - `nzcnt == 1`: 指向 `cb->__err`。`cb` 所指向的 `struct aiocb` 在 `aio_return` 被调用前必须保持有效（POSIX 要求），因此地址在等待期间有效。
   - `nzcnt >= 2`: 指向全局变量 `__aio_fut`（始终有效）。

2. **无竞态遗漏通知**: 在注册等待者身份后（对 `__aio_fut` 执行 CAS 后），函数重新扫描 `cbs` 数组。这保证：若某个 I/O 操作恰好在注册等待者与进入 futex 等待之间完成，其 `cleanup` 路径中的 `a_swap(&__aio_fut, 0)` 和 `__wake` 要么在重新扫描前发生（被重新扫描检测到并返回 0），要么在 `__timedwait_cp` 进入等待后发生（被 futex 唤醒）。

3. **EINPROGRESS 编码约定**: `__err` 字段的低 31 位存储实际错误码，高位（bit 31）用作 "有等待者" 标志。`aio_error()` 通过 `& 0x7fffffff` 掩码读取，因此等待者的存在对 `aio_error()` 结果透明。
```
