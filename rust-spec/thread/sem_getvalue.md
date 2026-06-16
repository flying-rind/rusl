# sem_getvalue — Rust 接口归约

## 原始 C 接口
```c
int sem_getvalue(sem_t *restrict sem, int *restrict valp);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_getvalue(sem: *mut sem_t, valp: *mut core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图

获取信号量的当前计数值。直接读取内部计数字段并使用位掩码 `SEM_VALUE_MAX` 提取有效值部分（低 31 位），忽略符号位（用于等待队列标记）。

## 前置条件

- `sem` 为非空指针，指向有效的 `sem_t`
- `valp` 为非空指针，指向可写的 `c_int` 变量

## 后置条件

- `*valp` 被设置为信号量的当前计数值（0 到 `SEM_VALUE_MAX` 之间）
- 返回 `0`

## 不变量

- 始终返回 `0`（POSIX 规定此函数不返回错误）
- 返回值为瞬时快照，调用后可能立即改变

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部使用 Rust 的原子读取获取计数值。

```rust
// extern "C" 函数内部流程
// sem_getvalue(sem, valp):
//   1. 原子读取 sem.__val[0]（即 sem.count 字段）
//   2. *valp = (val & SEM_VALUE_MAX) as c_int  // 屏蔽 bit 31 等待标记，提取计数值
//   3. 返回 0
```

内部改进要点：
- 使用 `core::sync::atomic::AtomicI32` 的 `load(Ordering::Acquire)` 替代直接内存读取
- 位掩码操作通过 Rust 的常量 `SEM_VALUE_MAX`（0x7FFFFFFF_u32）进行

位编码说明（与 C 实现一致）：
- 低 31 位（`& 0x7FFFFFFF`）：信号量计数值
- 第 31 位（bit 31）：内部等待标记

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供 crate 内部使用，接收 &Sem 引用）
pub(crate) fn sem_get_value(sem: &Sem) -> core::ffi::c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  sem_t / Sem                                        // 依赖1: 信号量类型（内部含原子计数字段）
  SEM_VALUE_MAX (0x7FFFFFFF)                         // 依赖2: 信号量最大值常量
Predefined Macros/Traits:
  core::sync::atomic::Ordering                       // 依赖3: 原子操作内存顺序

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_getvalue(sem: *mut sem_t, valp: *mut core::ffi::c_int) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn sem_get_value(sem: &Sem) -> core::ffi::c_int;
