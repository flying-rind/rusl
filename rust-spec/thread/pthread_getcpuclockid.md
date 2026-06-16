# pthread_getcpuclockid — Rust 接口归约

> rusl 获取指定线程的 CPU 时间时钟 ID。通过线程的内核 TID 计算出对应的 `clockid_t`。

---

## 原始 C 对外导出接口

```c
// 用户可见符号
int pthread_getcpuclockid(pthread_t t, clockid_t *clockid);
```

---

## Rust 外部 ABI 接口

```rust
// clockid_t 在 Linux 上为 int 类型，使用 c_int 保持 ABI 兼容
// 用户可见符号 — 与 C ABI 兼容
extern "C" fn pthread_getcpuclockid(
    t: *mut core::ffi::c_void,
    clockid: *mut core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图

根据给定线程 `t`，将其内核 TID 转换为对应的 CPU 时钟 ID，写入 `*clockid`，供 `clock_gettime` 使用。Linux 内核中每个线程的 CPU 时钟 ID 由 `CLOCK_THREAD_CPUTIME_ID` 宏编码计算得到。

**时钟 ID 编码公式**（Linux 内核约定）：
```
clockid = (-tid - 1) * 8 + 6
```
等价于：
```
clockid = (!tid) * 8 + 6
```

其中 `tid` 为内核线程 ID（通过 `gettid()` 获取），`-tid - 1` 在二进制补码中等价于按位取反。

---

## 前置条件

- `t` 不为 null，为有效的 `pthread_t`（指向一个已初始化的 `struct pthread`）
- `t->tid` 为有效的内核线程 ID（非零值）
- `clockid` 不为 null，指向可写入 `clockid_t` 的内存

---

## 后置条件

- `*clockid` 被设置为 `(-(t->tid) - 1) * 8 + 6`，即该线程的 CPU 时钟 ID
- 始终返回 0（成功）
- 无副作用：不修改线程或全局状态

---

## 不变量

无。该函数仅读取 `t->tid` 并写入 `*clockid`，不修改任何共享状态。

---

## 算法

```
pthread_getcpuclockid(t, clockid):
  1. *clockid = (-t.tid - 1) * 8 + 6
  2. return 0
```

Rust 内部实现：

```rust
use core::ffi::c_int;

const CLOCK_THREAD_CPUTIME_SHIFT: c_int = 3;  // 乘法因子 8 = 1 << 3
const CLOCK_THREAD_CPUTIME_OFFSET: c_int = 6;

pub extern "C" fn pthread_getcpuclockid(
    t: *mut core::ffi::c_void,
    clockid: *mut c_int,
) -> c_int {
    // 内部: 从 opaque pthread_t 获取 tid 字段
    let tid = unsafe { get_thread_tid(t) };
    unsafe {
        // clockid = (-tid - 1) * 8 + 6
        *clockid = (-tid - 1) * (1 << CLOCK_THREAD_CPUTIME_SHIFT) + CLOCK_THREAD_CPUTIME_OFFSET;
    }
    0
}
```

内部可封装为安全函数供 crate 内使用：

```rust
/// 安全版本: 从线程引用计算 CPU 时钟 ID
pub(crate) fn get_cpu_clock_id(t: &Pthread) -> c_int {
    (-t.tid - 1) * (1 << CLOCK_THREAD_CPUTIME_SHIFT) + CLOCK_THREAD_CPUTIME_OFFSET
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::{c_int, c_void}                               // 依赖1: C ABI 基本类型
  struct pthread (pthread_impl.h)                          // 依赖2: 线程结构体（访问 tid 字段）

Predefined Macros/Constants:
  CLOCK_THREAD_CPUTIME_ID 编码常量                          // Linux 内核 CPU 时钟 ID 编码公式

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_getcpuclockid(t: *mut core::ffi::c_void,
                                      clockid: *mut core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_getcpuclockid 符号

Internal Interface:
  pub(crate) fn get_cpu_clock_id(t: &Pthread) -> core::ffi::c_int;
                                 // 安全包装，供 crate 内部使用
