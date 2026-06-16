# pthread_mutexattr_init — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutexattr_init(pthread_mutexattr_t *a);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutexattr_init(a: *mut pthread_mutexattr_t) -> core::ffi::c_int;
```

---

## 类型定义

### pthread_mutexattr_t（Rust 侧表示）

```rust
#[repr(C)]
pub struct pthread_mutexattr_t {
    pub(crate) __attr: core::ffi::c_uint,
}
```

`__attr` 字段编码互斥锁属性位掩码，各位含义如下：

| 位 | 值 | 含义 |
|----|-----|------|
| 0–1 ( `& 3` ) | — | mutex type: 0=NORMAL/DEFAULT, 1=RECURSIVE, 2=ERRORCHECK |
| 2 ( `& 4` ) | `PTHREAD_MUTEX_ROBUST` | 健壮性标志 |
| 3 ( `& 8` ) | `PTHREAD_PRIO_INHERIT` | 优先级继承标志 |
| 7 ( `& 128` ) | `PTHREAD_PROCESS_SHARED` | 进程共享标志 |

默认值对应：`NORMAL` 类型、非 robust、非 PI、进程私有。

---

## 意图
将互斥锁属性对象 `a` 初始化为默认值。默认互斥锁为非递归、非健壮、非优先级继承的进程私有 NORMAL 互斥锁。

## 前置条件
- `a` 非空指针（`!a.is_null()`）
- `a` 指向调用者分配的 `pthread_mutexattr_t` 内存

## 后置条件
- Case 1（总是成功）：
  - `(*a).__attr == 0`
  - 返回值为 `0`

## 不变量
无。

## 算法
直接对属性结构体零初始化：

```rust
extern "C" fn pthread_mutexattr_init(a: *mut pthread_mutexattr_t) -> core::ffi::c_int {
    if a.is_null() {
        // 未定义行为保护，musl 不做此检查但安全 Rust 中可选加入
    }
    unsafe { core::ptr::write(a, pthread_mutexattr_t { __attr: 0 }); }
    0
}
```

内部可提供安全的 `Default` trait 实现：

```rust
impl Default for pthread_mutexattr_t {
    fn default() -> Self {
        Self { __attr: 0 }
    }
}
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
pub(crate) fn mutexattr_init() -> pthread_mutexattr_t {
    pthread_mutexattr_t::default()
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)                         // 无外部依赖，仅需结构体零初始化

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutexattr_init(a: *mut pthread_mutexattr_t) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutexattr_init 符号
Internal Interface:
  pub(crate) fn mutexattr_init() -> pthread_mutexattr_t;
                                 // 安全包装，返回默认属性，供 crate 内部使用
  impl Default for pthread_mutexattr_t;
                                 // Default trait 实现，供内部零初始化
