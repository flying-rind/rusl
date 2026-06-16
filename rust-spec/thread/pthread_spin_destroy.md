# pthread_spin_destroy — Rust 接口归约

## 原始 C 接口
```c
int pthread_spin_destroy(pthread_spinlock_t *s);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// pthread_spinlock_t 在 musl 中定义为 int
extern "C" fn pthread_spin_destroy(s: *mut core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
销毁自旋锁对象。由于 musl 中 `pthread_spinlock_t` 定义为 `int`，销毁后无需任何资源清理操作，为立即返回 0 的空操作。

## 前置条件
- `s` 为非空指针（`!s.is_null()`），指向先前通过 `pthread_spin_init` 初始化的自旋锁对象
- 该自旋锁未被任何线程持有

## 后置条件
- 总是返回 0（成功）
- 自旋锁对象 `*s` 不再可用，后续对其使用行为未定义

## 不变量
无。自旋锁仅为 `c_int` 类型值，无需释放资源。

## 算法

```rust
// 直接返回 0，空操作
// 注意：C 原实现中参数 shared 被忽略，仅设置 *s = 0 并返回 0
// Rust 中通过 extern "C" 包装提供相同语义
pub extern "C" fn pthread_spin_destroy(_s: *mut core::ffi::c_int) -> core::ffi::c_int {
    0
}
```

对 C 调用者：
1. `extern "C" fn pthread_spin_destroy(s: *mut c_int) -> c_int` 接收裸指针
2. 内部不作任何操作，立即返回 0

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
// 由于销毁是空操作，安全包装仅接受引用以验证生命周期但不执行任何清理
pub(crate) fn spin_destroy(_s: &core::sync::atomic::AtomicI32) {}
```

---

/* Rely */
[RELY]
Predefined Types:
  core::ffi::c_int                // 依赖1: C ABI 兼容的 int 类型
Predefined Macros/Traits:
  (none)                          // 无内部依赖

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_spin_destroy(s: *mut core::ffi::c_int) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 pthread_spin_destroy 符号
Internal Interface:
  pub(crate) fn spin_destroy(s: &core::sync::atomic::AtomicI32);
                                  // 安全包装，供 crate 内部使用（空操作）
