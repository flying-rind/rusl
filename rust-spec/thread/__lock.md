# __lock — Rust 接口归约

> rusl 内部自旋锁实现。将锁标志位和拥塞计数器编码到一个 `c_int` 中，通过原子操作实现高效的线程互斥。用于保护 libc 内部共享数据结构。Rust 实现使用 `AtomicI32` 替代 C 的 `volatile int` + `a_cas`/`a_fetch_add`，通过 futex 系统调用实现等待/唤醒。

## 原始 C 接口

```c
void __lock(volatile int *l);
void __unlock(volatile int *l);
```

[Visibility]: Internal — musl 内部锁原语，通过 hidden 符号导出供其他模块调用

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
pub extern "C" fn __lock(l: *mut core::ffi::c_int);
pub extern "C" fn __unlock(l: *mut core::ffi::c_int);
```

---

## 依赖图

```
__lock
  ├─> core::sync::atomic::AtomicI32     (Rust 标准原子类型)
  ├─> core::sync::atomic::Ordering     (内存排序)
  ├─> linux syscall: futex(FUTEX_WAIT) (Linux futex 系统调用)
  └─> libc.need_locks                  (全局锁状态)

__unlock
  ├─> core::sync::atomic::AtomicI32
  └─> linux syscall: futex(FUTEX_WAKE) (Linux futex 唤醒)
```

---

## 函数规约

### 1. __lock

```rust
pub extern "C" fn __lock(l: *mut core::ffi::c_int);
```

#### Intent

实现一种结合锁标志和拥塞计数的高效自旋锁。锁的状态编码在一个 `c_int` 中：`x == 0` 表示未锁定且无竞争；`x < 0` 表示已锁定，拥塞数 = `x - INT_MIN`；`x > 0` 表示未锁定但有拥塞计数。采用三级策略：快速路径（原子 CAS）、中等竞争自旋、重度竞争 futex 等待。

Rust 实现中将 `*mut c_int` 内部 reinterpret 为 `&AtomicI32`，使用 `compare_exchange` 替代 `a_cas`，`fetch_add` 替代 `a_fetch_add`。

#### 前置条件

- `l` 非空，指向有效的锁变量
- `libc` 全局结构体已正确初始化
- 调用者不在同一锁的临界区内（非递归锁）

#### 后置条件

- Case 1 单线程模式 (`!libc.need_locks`)：直接返回，不获取锁
- Case 2 快速路径成功 (CAS 成功)：锁已获取（值 < 0），返回
- Case 3 自旋成功：通过自旋重试 CAS 获取锁，返回
- Case 4 重度竞争：进入 futex 等待循环，最终通过 CAS 获取锁，返回
- 所有成功分支均保证：返回时值 < 0，调用者持有锁

#### 系统算法

```
__lock(l):
  1. if (!libc.need_locks) return
  2. 将 l 转为 &AtomicI32
  3. current = l.compare_exchange(0, INT_MIN + 1, AcqRel, Relaxed)
  4. if (!current) return                           // CAS 成功
  5. for i in 0..9:                                  // 中等竞争自旋
  6.   if (current < 0) current -= INT_MIN + 1
  7.   val = l.compare_exchange(current, INT_MIN + (current + 1), AcqRel, Relaxed)
  8.   if (val == current) return
  9.   current = val
 10. current = l.fetch_add(1, Relaxed) + 1           // 登记拥塞
 11. loop:                                           // 重度竞争 futex 等待
 12.   if (current < 0):
 13.     futex(FUTEX_WAIT, l, current, ...)          // 系统调用等待锁释放
 14.     current -= INT_MIN + 1
 15.   val = l.compare_exchange(current, INT_MIN + current, AcqRel, Relaxed)
 16.   if (val == current) return
 17.   current = val
```

#### 不变量

- 锁的状态始终由原子变量的值一致地表达
- 拥塞计数器始终反映等待或持有锁的线程数
- 锁的符号位 (`INT_MIN`) 始终唯一表示"锁定"状态

---

### 2. __unlock

```rust
pub extern "C" fn __unlock(l: *mut core::ffi::c_int);
```

#### Intent

释放由 `__lock` 获取的锁。检查是否有等待者，如果有则通过 futex wake 唤醒其中一个。是 `__lock` 的配对释放函数。

#### 前置条件

- `l` 非空，指向有效的锁变量
- 调用者当前持有该锁（值为负）

#### 后置条件

- 若值 >= 0（无竞争），直接返回
- 若值 < 0（有锁标志）：
  - 通过 `fetch_add` 原子地释放锁并减少拥塞计数
  - 若减法后结果不等于 `INT_MIN + 1`（还有其他等待者），调用 futex wake 唤醒一个等待者
- 调用者不再持有锁

#### 系统算法

```
__unlock(l):
  1. 将 l 转为 &AtomicI32
  2. if (l.load(Relaxed) < 0):
  3.   if (l.fetch_add(-(INT_MIN + 1), Release) != INT_MIN + 1):
  4.     futex(FUTEX_WAKE, l, 1, ...)               // 唤醒一个等待者
```

#### 不变量

- 释放操作是原子的：`fetch_add` 保证锁状态的一致转换
- 仅在确实存在等待者时才执行 futex wake

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的内部锁包装，使用 AtomicI32 + futex
struct InternalLock {
    state: core::sync::atomic::AtomicI32,
}

impl InternalLock {
    pub(crate) fn lock(&self) { /* ... */ }
    pub(crate) fn unlock(&self) { /* ... */ }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32          // 依赖1: 原子 I32 类型
  core::sync::atomic::Ordering           // 依赖2: 内存排序语义
  linux futex syscall (FUTEX_WAIT/FUTEX_WAKE) // 依赖3: Linux futex 系统调用
  libc.need_locks                        // 依赖4: 全局多线程标志

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __lock(l: *mut core::ffi::c_int);
                                         // 本模块保证对外提供与 C ABI 兼容的 __lock 符号
  pub extern "C" fn __unlock(l: *mut core::ffi::c_int);
                                         // 本模块保证对外提供与 C ABI 兼容的 __unlock 符号
Internal Interface:
  struct InternalLock { state: AtomicI32 }
  impl InternalLock { pub(crate) fn lock(&self); pub(crate) fn unlock(&self); }
                                         // 安全包装，供 crate 内部使用
