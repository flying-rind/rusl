# pthread_attr_init — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_init(pthread_attr_t *a);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_init(a: *mut pthread_attr_t) -> core::ffi::c_int;
```

---

## 意图

初始化线程属性对象，将属性对象清零后填入模块默认的栈大小和守护页大小。使用 PTC 锁保证读取全局默认值时的线程安全性。

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向一个未初始化的 `pthread_attr_t` 内存区域

## 后置条件

- Case 1 始终成功：返回 `0`
- 操作步骤：
  1. `*a` 全部字段清零（等价于 `(pthread_attr_t){0}`）
  2. `acquire_ptc()` — 获取 PTC 锁，确保原子读取全局默认值
  3. `a._a_stacksize` = `default_stacksize()` — 填入模块默认栈大小
  4. `a._a_guardsize` = `default_guardsize()` — 填入模块默认守护页大小
  5. `release_ptc()` — 释放 PTC 锁

## 不变量

- 函数执行期间 PTC 锁被持有，保证 `default_stacksize` / `default_guardsize` 的读取一致性
- `pthread_attr_t` 结构体其余成员保持零值（`detach`=0/joinable, `sched`=0/inherit, `policy`=0, `prio`=0, `stackaddr`=0）

## 算法

```
pthread_attr_init(a):
  1. 通过 write_volatile 或 ptr::write 将 a 指向的内存以零填充
     // Rust 中可用 core::ptr::write_bytes(a, 0, 1) 或解引用赋零值
  2. acquire_ptc() — 获取 PTC 锁
  3. (*a)._a_stacksize = default_stacksize()
  4. (*a)._a_guardsize = default_guardsize()
  5. release_ptc() — 释放 PTC 锁
  6. return 0
```

Rust 实现中，`pthread_attr_t` 应为 `#[repr(C)]` 结构体。清零可通过 `core::ptr::write(a, core::mem::zeroed())` 或逐字段赋零值实现。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t            // 定义于 pthread_impl 模块
  acquire_ptc()             // PTC 锁获取 (定义于 pthread_impl 模块)
  release_ptc()             // PTC 锁释放 (定义于 pthread_impl 模块)
  default_stacksize()       // 默认栈大小读取 (定义于 default_attr 模块)
  default_guardsize()       // 默认守护页大小读取 (定义于 default_attr 模块)
Predefined Macros/Constants:
  (无)

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_attr_init(a: *mut pthread_attr_t) -> core::ffi::c_int;
  // 将 a 指向的属性对象初始化为默认值，始终返回 0
Internal Interface:
  (无内部接口)
