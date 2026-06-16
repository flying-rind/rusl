# pthread_rwlockattr_destroy -- Rust 接口归约

## 原始 C 接口

```c
int pthread_rwlockattr_destroy(pthread_rwlockattr_t *a);
```

---

## Rust 外部 ABI 接口

```rust
// musl 中该函数无 __ 前缀的主实现，直接以 pthread_rwlockattr_destroy 导出
pub extern "C" fn pthread_rwlockattr_destroy(a: *mut pthread_rwlockattr_t) -> c_int;
```

---

## 意图

销毁读写锁属性对象 `a`。musl 实现中该结构体不持有动态分配的资源，因此该函数仅返回 0。

## 前置条件

- `a` 指向一个已初始化的有效 `pthread_rwlockattr_t` 对象
- 在销毁后不应再次使用 `a`，除非重新初始化

## 后置条件

- Case 1 成功: 返回 0
- 无错误分支（musl 实现总是返回 0）

## 不变量

无。

## 算法

```
pthread_rwlockattr_destroy(a):
  1. return 0
```

Rust 实现中直接返回 0，无需任何资源清理。

---

## 数据结构

### pthread_rwlockattr_t

```rust
#[repr(C)]
pub struct pthread_rwlockattr_t {
    pub __attr: [c_uint; 2],
}
```

读写锁属性对象，包含两个 unsigned int 字段：
- `__attr[0]`: 进程共享属性 (`PTHREAD_PROCESS_PRIVATE` = 0, `PTHREAD_PROCESS_SHARED` = 1)
- `__attr[1]`: 保留

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)                         // 无内部依赖，直接返回 0

Predefined Types:
  pthread_rwlockattr_t           // 来自 crate 内部类型定义，需保持 #[repr(C)] ABI 兼容

[GUARANTEE]
Exported Interface:
  pub extern "C" fn pthread_rwlockattr_destroy(a: *mut pthread_rwlockattr_t) -> c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_rwlockattr_destroy 符号
                                 // 总是返回 0
