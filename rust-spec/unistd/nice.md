# nice — Rust 接口归约

## 原始 C 接口
```c
int nice(int inc);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn nice(inc: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
将调用进程的调度优先级（nice 值）增加 `inc`。nice 值范围通常为 `[-NZERO, NZERO-1]`（即 [-20, 19]），较小的值（负值）意味着较高的调度优先级。只有具有适当权限的进程（通常是 root）才能将 nice 值降低（提高优先级）。

该实现使用 `getpriority`/`setpriority` 而非已弃用的 `SYS_nice` 系统调用。包含一个优化：当 `inc` 超出范围时跳过 `getpriority` 查询，直接进行边界裁剪。

## 前置条件
- `inc`: 要增加到当前 nice 值的增量（可以为负）

## 后置条件
- **Case 1 成功（有权限）**
  - 进程的 nice 值增加 `inc`，并被裁剪到 `[-NZERO, NZERO-1]` 范围内
  - 返回新的 nice 值（可能不同于 `old_nice + inc`，因为经过了边界裁剪）

- **Case 2 成功（无权限、试图降低 nice 值为负）**
  - nice 值不变
  - 返回新的 nice 值（即原始值，因为 `setpriority` 允许非特权进程提高 nice 值但忽略降低请求）

- **Case 3 错误（权限不足无法调用 setpriority）**
  - nice 值不变
  - 返回 -1
  - `errno` 设置为 `EPERM`（musl 将内核返回的 `EACCES` 映射为 `EPERM`，以符合 POSIX 规范）

- **注意**：由于 -1 也是合法的 nice 返回值（nice 值 -1 表示优先级为 -1），调用者应通过"调用前将 `errno` 设为 0，调用后检查 `errno != 0`"的模式来区分成功和失败。

## 不变量
- nice 值始终在 `[-NZERO, NZERO-1]` 范围内

## 算法
原 C 实现使用 `getpriority(PRIO_PROCESS, 0)` 查询当前 nice 值，通过 `setpriority(PRIO_PROCESS, 0, prio)` 设置新值，并将 `EACCES` 映射为 `EPERM`。Rust 中：

```
nice(inc):
  prio = inc                      // 1. 初始化目标增量为 inc
  if inc > -2*NZERO && inc < 2*NZERO:  // 2. 仅当增量在合理范围内才查询旧值
    prio += getpriority(PRIO_PROCESS, 0)  //   避免溢出和无效系统调用
  if prio > NZERO-1: prio = NZERO-1       // 3. 上界裁剪
  if prio < -NZERO:  prio = -NZERO        // 4. 下界裁剪
  if setpriority(PRIO_PROCESS, 0, prio):  // 5. 设置新优先级
    if errno == EACCES: errno = EPERM     // 6. 映射错误码
    return -1
  return prio                              // 7. 返回新 nice 值
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  getpriority(int, id_t)              // 依赖1: 获取进程调度优先级
  setpriority(int, id_t, int)         // 依赖2: 设置进程调度优先级
  PRIO_PROCESS                        // 依赖3: 进程优先级类型宏
  NZERO                               // 依赖4: 默认优先级常量
  EACCES, EPERM                       // 依赖5: 错误码
Predefined Macros/Crates:
  libc crate（或等效 no_std crate）   // 依赖6: 提供 getpriority/setpriority

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn nice(inc: core::ffi::c_int) -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 nice 符号
