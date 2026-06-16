# pthread_mutex_init — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutex_init(pthread_mutex_t *restrict m, const pthread_mutexattr_t *restrict a);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutex_init(
    m: *mut pthread_mutex_t,
    a: *const pthread_mutexattr_t
) -> core::ffi::c_int;
```

---

## 互斥锁类型布局

Rust 侧 `pthread_mutex_t` 定义为 `#[repr(C)]` union，字段通过以下访问器暴露：

```rust
#[repr(C)]
pub union pthread_mutex_t {
    // 64-bit: __i[10], __vi[10], __p[5]
    // 32-bit: __i[6],  __vi[6],  __p[6]
    // 内部字段按平台选择
}

// 访问器宏对应方法
impl pthread_mutex_t {
    fn get_type(&self) -> core::ffi::c_int;       // _m_type    — 类型/属性位掩码
    fn get_lock(&self) -> core::ffi::c_int;         // _m_lock    — 锁状态 (owner tid / 标志位)
    fn set_lock(&mut self, v: core::ffi::c_int);
    fn get_waiters(&self) -> core::ffi::c_int;      // _m_waiters — 等待者计数/标志
    fn get_prev_ptr(&self) -> ...;                  // _m_prev    — robust list 前驱指针
    fn get_next_ptr(&self) -> ...;                  // _m_next    — robust list 后继指针
    fn get_count(&self) -> core::ffi::c_int;        // _m_count   — 递归计数
    fn set_count(&mut self, v: core::ffi::c_int);
}
```

默认初始化器将所有字段初始化为零。

---

## 意图
根据可选的属性对象初始化互斥锁。若属性对象为 NULL，则使用默认属性（NORMAL 类型、非递归、非健壮、进程私有）。

## 前置条件
- `m` 非空指针（`!m.is_null()`）
- `m` 指向调用者分配的 `pthread_mutex_t` 内存
- 若 `a` 非空，则 `a` 指向一个已初始化的 `pthread_mutexattr_t`

## 后置条件
- Case 1（总是成功）：
  - `m` 的所有 union 字段被零初始化
  - 若 `a` 非空：`(*m)._m_type = (*a).__attr`
  - 若 `a` 为空：`(*m)._m_type = 0`（默认属性）
  - 返回值为 `0`

## 不变量
- 初始化后的互斥锁处于未加锁状态（`_m_lock == 0`）
- `_m_count == 0`（递归计数归零）
- `_m_waiters == 0`（无等待者）
- `_m_prev == NULL` / `_m_next == NULL`（未加入 robust list）

## 算法
直接零初始化结构体并选择性设置 type 字段：

```rust
extern "C" fn pthread_mutex_init(
    m: *mut pthread_mutex_t,
    a: *const pthread_mutexattr_t
) -> core::ffi::c_int {
    unsafe {
        // 零初始化全部 union 字段
        core::ptr::write(m, core::mem::zeroed());
        // 设置可选属性
        if !a.is_null() {
            (*m).set_type((*a).__attr as core::ffi::c_int);
        }
    }
    0
}
```

内部可提供构造器方法，从 `&PthreadMutexattr` 安全构建 `PthreadMutex`：

```rust
impl PthreadMutex {
    pub(crate) fn new(attr: Option<&PthreadMutexattr>) -> Self {
        let mut m = Self::zeroed();
        if let Some(a) = attr {
            m.set_type(a.__attr as core::ffi::c_int);
        }
        m
    }
}
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
impl PthreadMutex {
    pub(crate) fn new(attr: Option<&PthreadMutexattr>) -> Self;
    pub(crate) fn zeroed() -> Self;
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::write               // 依赖: 安全写入已分配内存
  core::mem::zeroed              // 依赖: 零初始化

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutex_init(m: *mut pthread_mutex_t, a: *const pthread_mutexattr_t) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_init 符号
Internal Interface:
  impl PthreadMutex::new(attr: Option<&PthreadMutexattr>) -> Self;
                                 // 安全构造器，供 crate 内部使用
  impl PthreadMutex::zeroed() -> Self;
                                 // 零初始化构造器
