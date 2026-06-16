# pthread_rwlock_destroy -- Rust 接口归约

## 原始 C 接口

```c
int pthread_rwlock_destroy(pthread_rwlock_t *rw);
```

---

## Rust 外部 ABI 接口

```rust
// musl 中该函数无 __ 前缀的主实现，直接以 pthread_rwlock_destroy 导出
pub extern "C" fn pthread_rwlock_destroy(rw: *mut pthread_rwlock_t) -> c_int;
```

---

## 意图

销毁读写锁对象 `rw`，释放其占用的实现定义资源。musl 实现中读写锁不持有内核资源或动态分配的内存，因此仅返回 0。

## 前置条件

- `rw` 指向一个已初始化的 `pthread_rwlock_t` 对象
- 调用时没有线程持有或等待该读写锁（未定义行为否则）
- 销毁后不应再次使用 `rw`，除非重新初始化

## 后置条件

- Case 1 成功: 返回 0
- 无错误分支（musl 实现总是返回 0）

## 不变量

无。

## 算法

```
pthread_rwlock_destroy(rw):
  1. return 0
```

Rust 实现中直接返回 0，无需任何资源清理。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)                         // 无内部依赖，直接返回 0

Predefined Types:
  pthread_rwlock_t               // 来自 crate 内部类型定义，需保持 #[repr(C)] ABI 兼容

[GUARANTEE]
Exported Interface:
  pub extern "C" fn pthread_rwlock_destroy(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_rwlock_destroy 符号
                                 // 总是返回 0
