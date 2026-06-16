# vmlock — Rust 接口归约

> rusl 内部虚拟内存锁。保护 `fork()` 期间父进程的虚拟内存布局不被其他线程的 `mmap`/`munmap` 操作修改，防止子进程看到不一致的内存映射状态。Rust 实现中使用 `AtomicI32` 替代 `volatile int` 数组。

## 原始 C 接口

```c
void __vm_wait(void);
void __vm_lock(void);
void __vm_unlock(void);
extern volatile int *const __vmlock_lockptr;
```

[Visibility]: Internal — 被 fork 相关代码调用，仅 musl/rusl 内部使用

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __vm_wait();
pub extern "C" fn __vm_lock();
pub extern "C" fn __vm_unlock();

// 全局可见指针，供 fork_impl 等访问
pub static __vmlock_lockptr: *const core::ffi::c_int;
```

---

## 依赖图

```
__vm_wait
  └─> __wait(vmlock, vmlock+1, tmp, 1)   (调用 __wait futex 等待)

__vm_lock
  └─> a_inc(vmlock) / AtomicI32::fetch_add(1)

__vm_unlock
  ├─> a_fetch_add(vmlock, -1)             / AtomicI32::fetch_sub(1)
  └─> __wake(vmlock, -1, 1)              (futex FUTEX_WAKE)
```

---

## 内部全局变量（不对外导出）

### vmlock

```rust
// 文件作用域静态变量，双元素数组实现排他锁
// vmlock[0]: 锁计数。0 = 未锁定，正数 = 有线程在虚拟内存操作中
// vmlock[1]: 等待者计数。非零表示有线程在等待锁释放
static VMLOCK: [core::sync::atomic::AtomicI32; 2] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
];
```

#### 不变量

- `VMLOCK[0]` 的值反映正在进行虚拟内存操作的线程数
- `VMLOCK[1]` 的值反映等待锁释放的线程数

---

## 函数规约

### 1. __vm_wait

```rust
pub extern "C" fn __vm_wait();
```

#### Intent

等待虚拟内存锁变为可用（即 `VMLOCK[0] == 0`）。当线程需要进行虚拟内存操作但锁被占用时调用。通过 `__wait` 在 `vmlock` 上进行 futex 等待，并维护等待者计数。

#### 前置条件

- `VMLOCK[0]` 可能非零（有其他线程持有 VM 锁）

#### 后置条件

- 返回时 `VMLOCK[0] == 0`（锁已释放）
- 等待期间，此线程通过 futex 阻塞

#### 系统算法

```
__vm_wait():
  1. while ((tmp = VMLOCK[0].load(Relaxed)) != 0):
  2.   __wait(&VMLOCK[0] as *mut c_int, &VMLOCK[1] as *mut c_int, tmp, 1)
```

---

### 2. __vm_lock

```rust
pub extern "C" fn __vm_lock();
```

#### Intent

获取虚拟内存锁。原子递增 `VMLOCK[0]`，阻止其他线程在 `fork()` 期间修改虚拟内存映射。允许多个线程同时持有（引用计数语义）。

#### 前置条件

- 无特殊前置条件（原子操作在任何状态下安全）

#### 后置条件

- `VMLOCK[0]` 原子递增 1
- 调用者被视为"持有" VM 锁（但允许多个持有者共存）

#### 系统算法

```
__vm_lock():
  1. VMLOCK[0].fetch_add(1, Ordering::Relaxed)
```

---

### 3. __vm_unlock

```rust
pub extern "C" fn __vm_unlock();
```

#### Intent

释放虚拟内存锁。原子递减 `VMLOCK[0]`。若递减后锁计数为 0（最后一个持有者释放）且有等待者，则唤醒所有等待者。

#### 前置条件

- 调用者此前已调用 `__vm_lock()`（或通过其他方式确保锁计数 > 0）

#### 后置条件

- `VMLOCK[0]` 原子递减 1
- 若递减前值为 1（最后一个持有者释放）且 `VMLOCK[1]` 非零（有等待者）：
  - 调用 futex FUTEX_WAKE 唤醒所有等待者

#### 系统算法

```
__vm_unlock():
  1. if (VMLOCK[0].fetch_sub(1, Ordering::Release) == 1 && VMLOCK[1].load(Relaxed) != 0):
  2.   futex(FUTEX_WAKE, &VMLOCK[0], -1, ...)  // 唤醒所有等待者
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicI32, Ordering};

/// 内部 VM 锁结构，封装双元素原子数组
struct VmLock {
    lock: AtomicI32,    // 索引 0: 锁计数
    waiters: AtomicI32, // 索引 1: 等待者计数
}

impl VmLock {
    pub(crate) const fn new() -> Self;
    pub(crate) fn wait(&self);    // 等待锁变为可用
    pub(crate) fn lock(&self);    // 获取锁（引用计数递增）
    pub(crate) fn unlock(&self);  // 释放锁（引用计数递减，必要时唤醒等待者）
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32          // 依赖1: 原子 I32 类型
  __wait()                                // 依赖2: futex 等待原语
  linux futex syscall (FUTEX_WAKE)       // 依赖3: futex 唤醒系统调用

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __vm_wait();
  pub extern "C" fn __vm_lock();
  pub extern "C" fn __vm_unlock();
  pub static __vmlock_lockptr: *const core::ffi::c_int;
                                         // 本模块保证对外提供与 C ABI 兼容的 VM 锁符号
Internal Interface:
  struct VmLock { lock: AtomicI32, waiters: AtomicI32 }
  impl VmLock { pub(crate) fn wait(&self); pub(crate) fn lock(&self); pub(crate) fn unlock(&self); }
                                         // 安全 VM 锁封装，供 crate 内部使用
