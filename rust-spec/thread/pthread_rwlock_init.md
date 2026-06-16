# pthread_rwlock_init -- Rust 接口归约

## 原始 C 接口

```c
int pthread_rwlock_init(pthread_rwlock_t *restrict rw, const pthread_rwlockattr_t *restrict a);
```

---

## Rust 外部 ABI 接口

```rust
// musl 中该函数无 __ 前缀的主实现，直接以 pthread_rwlock_init 导出
pub extern "C" fn pthread_rwlock_init(rw: *mut pthread_rwlock_t, a: *const pthread_rwlockattr_t) -> c_int;
```

---

## 意图

初始化读写锁对象 `rw`。若 `a` 为 NULL，使用默认属性（进程私有）；否则从 `a` 读取进程共享设置。

## 前置条件

- `rw` 为非空指针（`!rw.is_null()`），指向未初始化或可重新初始化的内存
- `a` 为 NULL 或指向已初始化的有效 `pthread_rwlockattr_t` 对象
- `rw` 和 `a` 不重叠（restrict 约束）

## 后置条件

- Case 1 成功:
  - `rw` 被零初始化: `_rw_lock = 0`, `_rw_waiters = 0`, `_rw_shared = 0`
  - 若 `a != NULL`: `rw._rw_shared = (*a).__attr[0] * 128`
  - 返回 0
- 无错误分支

## 不变量

- `_rw_lock` 初值为 0（锁空闲）
- `_rw_shared` 为 0 或 128，决定 futex 的 private 标志行为

## 算法

```
pthread_rwlock_init(rw, a):
  1. unsafe { core::ptr::write_bytes(rw, 0u8, 1) }    零初始化整个结构体
  2. if a != NULL:
       (*rw)._rw_shared = (*a).__attr[0] * 128          0 或 128
  3. return 0
```

`_rw_shared` 字段的编码约定:
- `0`: 进程共享模式（futex 操作使用 `FUTEX_WAIT` / `FUTEX_WAKE`）
- `128`: 进程私有模式（futex 操作使用 `FUTEX_WAIT | FUTEX_PRIVATE` / `FUTEX_WAKE | FUTEX_PRIVATE`）
- 在 `__wake`、`__timedwait` 调用中通过 `priv = rw._rw_shared ^ 128` 提取出 `0` 或 `128` 作为 futex private 标志

---

## 数据结构

### pthread_rwlock_t 内部字段

```rust
// 需保持 #[repr(C)] 与原 C 结构体 ABI 兼容
// C 结构中通过 union 和宏定义字段访问，Rust 中可封装为方法
//
// C 结构体定义:
// typedef struct {
//     union { int __i[...]; volatile int __vi[...]; void *__p[...]; } __u;
// } pthread_rwlock_t;
//
// #define _rw_lock    __u.__vi[0]   // 锁状态字
// #define _rw_waiters __u.__vi[1]   // 等待者计数
// #define _rw_shared  __u.__i[2]    // 进程共享标志
```

锁状态字 `_rw_lock` 语义:
- `0`: 未加锁
- `1` ~ `0x7ffffffe`: 持有读锁的线程数
- `0x7fffffff` (INT32_MAX): 写锁持有中
- bit 31 (`0x80000000`): 等待者标志（用于 futex wait 值编码）

```rust
// Rust 中可使用辅助方法访问字段:
impl pthread_rwlock_t {
    pub(crate) fn lock_val(&self) -> c_int { /* self.__u.__vi[0] */ }
    pub(crate) fn set_lock_val(&mut self, v: c_int) { /* self.__u.__vi[0] = v */ }
    pub(crate) fn waiters(&self) -> c_int { /* self.__u.__vi[1] */ }
    pub(crate) fn set_waiters(&mut self, v: c_int) { /* self.__u.__vi[1] = v */ }
    pub(crate) fn shared(&self) -> c_int { /* self.__u.__i[2] */ }
    pub(crate) fn set_shared(&mut self, v: c_int) { /* self.__u.__i[2] = v */ }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::write_bytes          // 依赖1: 零初始化内存块

Predefined Constants:
  PTHREAD_PROCESS_PRIVATE        // 值 = 0
  PTHREAD_PROCESS_SHARED         // 值 = 1

Predefined Types:
  pthread_rwlock_t               // 来自 crate 内部类型定义
  pthread_rwlockattr_t           // 来自 crate 内部类型定义

[GUARANTEE]
Exported Interface:
  pub extern "C" fn pthread_rwlock_init(rw: *mut pthread_rwlock_t, a: *const pthread_rwlockattr_t) -> c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_rwlock_init 符号
                                 // 零初始化 rw，从 a 读取共享属性（若 a 非 NULL），总是返回 0
