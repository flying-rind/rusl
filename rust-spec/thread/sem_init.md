# sem_init — Rust 接口归约

## 原始 C 接口
```c
int sem_init(sem_t *sem, int pshared, unsigned value);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_init(
    sem: *mut sem_t,
    pshared: core::ffi::c_int,
    value: core::ffi::c_uint
) -> core::ffi::c_int;
```

---

## 意图

初始化一个匿名信号量。设置初始计数值与进程共享标志。rusl 内部可使用 Rust 的 `AtomicI32` 字段和类型安全的方式初始化信号量。

## 前置条件

- `sem` 为非空指针，指向待初始化的 `sem_t`
- `value <= SEM_VALUE_MAX`（即 `<= 0x7FFFFFFF`）

## 后置条件

- Case 1 成功（`value <= SEM_VALUE_MAX`）：
  - 信号量计数值（`__val[0]` / `count` 原子字段）设为 `value`
  - 等待者计数（`__val[1]` / `waiters` 字段）设为 `0`
  - 私有/共享标志（`__val[2]`）编码 futex 标志：进程内为 `FUTEX_PRIVATE`（128），跨进程为 `0`
  - 返回 `0`
- Case 2 失败（`value > SEM_VALUE_MAX`）：
  - `errno = EINVAL`
  - 返回 `-1`

## 不变量

- `sem.__val[2]` 编码 futex 私有标志：`128`（`FUTEX_PRIVATE`）表示进程内信号量；`0` 表示进程共享信号量
- 匿名信号量与有名信号量使用相同的数据结构 `sem_t`，通过 `__val[2]` 区分

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部使用 Rust 的原子类型安全初始化。

```rust
// extern "C" 函数内部流程
// sem_init(sem, pshared, value):
//   1. 若 value > SEM_VALUE_MAX → 设置 errno = EINVAL，返回 -1
//   2. 初始化信号量内部字段：
//      - count.store(value, Ordering::Release)     // 计数值
//      - waiters.store(0, Ordering::Release)         // 等待者计数初始为 0
//      - futex_flags = if pshared == 0 { FUTEX_PRIVATE } else { 0 }
//   3. 返回 0
```

内部 Rust 类型设计建议：

```rust
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

// 信号量内部结构（与 C ABI 的 sem_t 内存布局兼容）
#[repr(C)]
pub(crate) struct SemInner {
    count:   AtomicI32,  // __val[0]: 计数值 + 等待标记位
    waiters: AtomicI32,  // __val[1]: 等待者计数
    flags:   u32,        // __val[2]: futex 私有/共享标志
    // futex 标志: FUTEX_PRIVATE (128) 或 0 (跨进程)
}

const FUTEX_PRIVATE: u32 = 128;
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（接收 &mut SemInner 引用）
pub(crate) fn sem_init_inner(sem: &mut SemInner, pshared: bool, value: u32) -> Result<(), core::ffi::c_int>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  sem_t / SemInner                                    // 依赖1: 信号量类型（含原子字段）
  SEM_VALUE_MAX (0x7FFFFFFF)                          // 依赖2: 信号量最大值常量
  FUTEX_PRIVATE (128)                                  // 依赖3: futex 私有标志
Predefined Macros/Traits:
  core::sync::atomic::{AtomicI32, AtomicU32, Ordering} // 依赖4: Rust 原子类型

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_init(sem: *mut sem_t, pshared: core::ffi::c_int, value: core::ffi::c_uint) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn sem_init_inner(sem: &mut SemInner, pshared: bool, value: u32) -> Result<(), core::ffi::c_int>;
