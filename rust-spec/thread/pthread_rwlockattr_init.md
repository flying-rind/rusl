# pthread_rwlockattr_init -- Rust 接口归约

## 原始 C 接口

```c
int pthread_rwlockattr_init(pthread_rwlockattr_t *a);
```

---

## Rust 外部 ABI 接口

```rust
// musl 中该函数无 __ 前缀的主实现，直接以 pthread_rwlockattr_init 导出
pub extern "C" fn pthread_rwlockattr_init(a: *mut pthread_rwlockattr_t) -> c_int;
```

---

## 意图

将读写锁属性对象 `a` 初始化为默认值。musl 实现中默认值为全零：`__attr[0] = 0` 表示 `PTHREAD_PROCESS_PRIVATE`。

## 前置条件

- `a` 为非空指针（`!a.is_null()`）
- `a` 指向未初始化或可重新初始化的 `pthread_rwlockattr_t` 内存

## 后置条件

- Case 1 成功:
  - `(*a).__attr[0] = 0`（默认进程私有）
  - `(*a).__attr[1] = 0`（保留字段）
  - 返回 0
- 无错误分支

## 不变量

无。

## 算法

```
pthread_rwlockattr_init(a):
  1. unsafe { core::ptr::write_bytes(a, 0u8, 1) }   零初始化整个结构体
  2. return 0
```

Rust 实现使用 `core::ptr::write_bytes` 对结构体做零初始化，等价于 C 的复合字面量 `(pthread_rwlockattr_t){0}`。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
pub(crate) fn rwlockattr_init(a: &mut pthread_rwlockattr_t) {
    a.__attr = [0, 0];
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::write_bytes          // 依赖1: 零初始化内存块

Predefined Types:
  pthread_rwlockattr_t           // 来自 crate 内部类型定义

[GUARANTEE]
Exported Interface:
  pub extern "C" fn pthread_rwlockattr_init(a: *mut pthread_rwlockattr_t) -> c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_rwlockattr_init 符号
                                 // 将 *a 零初始化，总是返回 0

Internal Interface:
  pub(crate) fn rwlockattr_init(a: &mut pthread_rwlockattr_t);
                                 // 安全包装，供 crate 内部使用
