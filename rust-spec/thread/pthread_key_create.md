# pthread_key_create — Rust 接口归约

## 原始 C 接口
```c
int __pthread_key_create(pthread_key_t *k, void (*dtor)(void *));
int __pthread_key_delete(pthread_key_t k);
int pthread_key_create(pthread_key_t *k, void (*dtor)(void *));
int pthread_key_delete(pthread_key_t k);
```

`pthread_key_create` 是 `__pthread_key_create` 的 weak_alias；`pthread_key_delete` 是 `__pthread_key_delete` 的 weak_alias。

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数（musl __ 前缀主实现 + 用户可见别名）
extern "C" fn __pthread_key_create(k: *mut pthread_key_t, dtor: Option<extern "C" fn(*mut core::ffi::c_void)>) -> core::ffi::c_int;
extern "C" fn pthread_key_create(k: *mut pthread_key_t, dtor: Option<extern "C" fn(*mut core::ffi::c_void)>) -> core::ffi::c_int;

extern "C" fn __pthread_key_delete(k: pthread_key_t) -> core::ffi::c_int;
extern "C" fn pthread_key_delete(k: pthread_key_t) -> core::ffi::c_int;
```

---

## 意图

管理线程局部存储（TSD）键的创建、删除和析构函数注册。全局维护 `PTHREAD_KEYS_MAX`（128）个槽位的析构函数表，支持最多 4 轮析构函数调用。rusl 内部可使用 Rust 安全抽象（如 `Mutex<KeysState>`）替代 C 的裸静态变量 + 读写锁。

## 前置条件

- `k` 为非空指针，指向有效的 `pthread_key_t`
- 若 `dtor == None`，内部用哨兵析构函数替代
- 对于 `__pthread_key_delete`，`k` 是先前通过 `pthread_key_create` 成功创建的键

## 后置条件

- Case 1 `pthread_key_create` 成功：`*k` 被设置为新分配的键值，返回 `0`
- Case 2 `pthread_key_create` 失败（128 个槽位已满）：返回 `EAGAIN`
- Case 3 `pthread_key_delete` 成功：所有线程的 `tsd[k]` 被清零，`keys[k]` 被标记为空闲，返回 `0`

## 不变量

- `keys` 表中 `None` 条目数 + `Some` 条目数 == `PTHREAD_KEYS_MAX`（128）
- `next_key` 始终是有效的键索引（0..127）
- 删除操作完成后，被删键相关的所有线程状态已清空

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数，内部实现可完全重新设计。

### 内部状态设计（Safe Rust）

```rust
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

const PTHREAD_KEYS_MAX: usize = 128;
const PTHREAD_DESTRUCTOR_ITERATIONS: usize = 4;

// 析构函数类型
type DtorFn = Option<extern "C" fn(*mut core::ffi::c_void)>;

// 哨兵析构函数（区分"槽位空闲"和"无析构"）
fn nodtor(_dummy: *mut core::ffi::c_void) {}

// 内部键管理状态（使用 Rust 同步原语保护）
struct KeysState {
    // TSD 键析构函数表：None 表示槽位空闲
    keys: [DtorFn; PTHREAD_KEYS_MAX],
    // 下次分配起始搜索位置
    next_key: usize,
}

// 线程退出时运行 TSD 析构函数（内部实现，不对外导出）
pub(crate) fn tsd_run_dtors(self_tsd: &mut [*mut core::ffi::c_void; PTHREAD_KEYS_MAX]) {
    for _ in 0..PTHREAD_DESTRUCTOR_ITERATIONS {
        // 遍历所有键，调用非空非哨兵析构函数
        // 在调用析构前先清零对应 TSD 值，防止重入
        // ... 具体实现 ...
    }
}
```

### extern "C" 函数内部流程

`__pthread_key_create(k, dtor)`:
1. 将 `dtor` 的 `None` 映射为哨兵析构函数
2. 获取 `keys_state` 的写锁
3. 从 `next_key` 起循环搜索空闲槽位（`keys[j] == None`）
4. 若找到空闲槽：设置 `keys[j]`，更新 `*k` 和 `next_key`，释放锁，返回 `0`
5. 若 128 个槽位全满：释放锁，返回 `EAGAIN`

`__pthread_key_delete(k)`:
1. 屏蔽应用程序信号
2. 获取 `keys_state` 写锁和线程列表锁
3. 遍历所有线程，将每个线程的 `tsd[k]` 设为 `null`
4. 标记 `keys[k] = None`（释放槽位）
5. 释放锁，恢复信号，返回 `0`

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// TSD 析构函数注册表（crate 内部共享状态）
pub(crate) struct KeyRegistry {
    // 使用 Rust 的同步原语替代 C 的 pthread_rwlock_t
    inner: crate::sync::RwLock<KeysState>,
}

pub(crate) fn key_create(dtor: DtorFn) -> Result<pthread_key_t, core::ffi::c_int>;
pub(crate) fn key_delete(k: pthread_key_t) -> core::ffi::c_int;
pub(crate) fn tsd_run_dtors(self_tsd: &mut [*mut core::ffi::c_void; PTHREAD_KEYS_MAX]);
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_self()                                   // 依赖1: 获取当前线程结构体
  crate::sync::RwLock                                // 依赖2: Rust 读写锁（替代 pthread_rwlock_t）
  __block_app_sigs() / __restore_sigs()              // 依赖3: 信号屏蔽/恢复
  __tl_lock() / __tl_unlock()                        // 依赖4: 线程列表锁（弱别名，真实现由 pthread_create 提供）
Predefined Macros/Traits:
  core::option::Option                               // 替代 NULL 指针的 Option 类型
  core::sync::atomic                                 // 原子操作

[GUARANTEE]
Exported Interface:
  extern "C" fn __pthread_key_create(k: *mut pthread_key_t, dtor: Option<extern "C" fn(*mut core::ffi::c_void)>) -> core::ffi::c_int;
  extern "C" fn pthread_key_create(k: *mut pthread_key_t, dtor: Option<extern "C" fn(*mut core::ffi::c_void)>) -> core::ffi::c_int;
  extern "C" fn __pthread_key_delete(k: pthread_key_t) -> core::ffi::c_int;
  extern "C" fn pthread_key_delete(k: pthread_key_t) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn tsd_run_dtors(self_tsd: &mut [*mut core::ffi::c_void; PTHREAD_KEYS_MAX]);
  pub(crate) fn key_create(dtor: DtorFn) -> Result<pthread_key_t, core::ffi::c_int>;
  pub(crate) fn key_delete(k: pthread_key_t) -> core::ffi::c_int;
