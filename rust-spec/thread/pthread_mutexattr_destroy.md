# pthread_mutexattr_destroy — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutexattr_destroy(pthread_mutexattr_t *a);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutexattr_destroy(a: *mut pthread_mutexattr_t) -> core::ffi::c_int;
```

---

## 意图
销毁互斥锁属性对象 `a`。musl 中属性对象不含动态分配内存，因此无需释放任何资源，该函数仅返回成功。在 Rust 实现中同样保持此语义：属性对象是调用者分配的内联结构体，不持有堆资源。

## 前置条件
- `a` 非空指针（`!a.is_null()`）
- `a` 指向一个已初始化的 `pthread_mutexattr_t`（通过 `pthread_mutexattr_init` 或等价方式）

## 后置条件
- Case 1（总是成功）：
  - 返回值为 `0`
  - `a` 指向的对象语义上已销毁，调用者不应再使用

## 不变量
无。

## 算法
由于 musl 实现为无操作（属性对象无动态分配资源），Rust 实现直接返回 0：

```rust
extern "C" fn pthread_mutexattr_destroy(a: *mut pthread_mutexattr_t) -> core::ffi::c_int {
    // musl 中属性对象不含堆资源，直接返回成功
    0
}
```

内部无需任何资源释放逻辑。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
pub(crate) fn mutexattr_destroy(a: &mut PthreadMutexattr) {
    // Rust 安全包装：drop 时无需操作，资源由调用者管理
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)                         // 无外部依赖

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutexattr_destroy(a: *mut pthread_mutexattr_t) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutexattr_destroy 符号
Internal Interface:
  pub(crate) fn mutexattr_destroy(a: &mut PthreadMutexattr);
                                 // 安全包装，供 crate 内部使用
