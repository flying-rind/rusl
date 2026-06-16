# pthread_equal — Rust 接口归约

## 原始 C 接口
```c
// 对外导出（POSIX），是 __pthread_equal 的弱别名
int pthread_equal(pthread_t a, pthread_t b);

// 对外导出（ISO C11），是 __pthread_equal 的弱别名
int thrd_equal(pthread_t a, pthread_t b);

// 内部 static 函数，统一的比较实现
static int __pthread_equal(pthread_t a, pthread_t b);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// pthread_t 在 musl 中为指针类型，比较即指针相等比较
// 必须同时导出所有符号

extern "C" fn pthread_equal(a: pthread_t, b: pthread_t) -> core::ffi::c_int;

extern "C" fn thrd_equal(a: pthread_t, b: pthread_t) -> core::ffi::c_int;
```

---

## 意图
比较两个线程标识符是否相等。`pthread_t` 是不透明类型，不可直接使用 `==` 比较（保证可移植性），必须通过此函数。

## 前置条件
- `a` 和 `b` 为有效的 `pthread_t` 值（来自 `pthread_self()` 或 `pthread_create()` 的输出参数）

## 后置条件
- 若 `a` 和 `b` 指向同一个线程，返回非零值（在 musl 中为 1）
- 若 `a` 和 `b` 指向不同线程，返回 0

## 不变量
无。

## 注意事项
- `<pthread.h>` 中同时提供了同名的宏定义 `#define pthread_equal(x,y) ((x)==(y))`，C++ 下不可用，由编译器/环境选择使用宏或函数
- musl 中 `pthread_t` 是指针，因此 `a == b` 即指针相等比较

## 算法

```rust
// pthread_t 类型定义
pub type pthread_t = *mut Thread;

// __pthread_equal — 内部统一实现
fn __pthread_equal(a: pthread_t, b: pthread_t) -> core::ffi::c_int {
    (a == b) as core::ffi::c_int  // 指针相等比较，true→1, false→0
}

// pthread_equal — POSIX 对外导出
pub extern "C" fn pthread_equal(a: pthread_t, b: pthread_t) -> core::ffi::c_int {
    __pthread_equal(a, b)
}

// thrd_equal — C11 对外导出
pub extern "C" fn thrd_equal(a: pthread_t, b: pthread_t) -> core::ffi::c_int {
    __pthread_equal(a, b)
}
```

对 C 调用者：
1. `extern "C" fn pthread_equal(a: pthread_t, b: pthread_t) -> c_int`
2. 内部执行指针相等比较 `a == b`
3. 返回非零（1）表示相同线程，0 表示不同线程

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装。由于 Rust 的 pthread_t 内部已经是指针，
// 直接使用 == 也是安全的。此包装提供更语义化的接口。

pub(crate) fn threads_equal(a: pthread_t, b: pthread_t) -> bool {
    a == b  // 指针相等
}

// 或者：为 ThreadHandle 实现 PartialEq
impl PartialEq for ThreadHandle {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
```

---

/* Rely */
[RELY]
Predefined Types:
  pthread_t                           // 依赖1: 线程标识符类型（musl 中为 *mut Thread）
Predefined Structures:
  Thread (struct __pthread)           // 依赖2: 线程内部结构体（仅类型引用）

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_equal(a: pthread_t, b: pthread_t) -> core::ffi::c_int;
  extern "C" fn thrd_equal(a: pthread_t, b: pthread_t) -> core::ffi::c_int;
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_equal 和 thrd_equal 符号
Internal Interface:
  pub(crate) fn threads_equal(a: pthread_t, b: pthread_t) -> bool;
                                    // 安全包装，供 crate 内部使用
