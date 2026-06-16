# pthread_setattr_default_np — Rust 接口归约

## 原始 C 接口

```c
int pthread_setattr_default_np(const pthread_attr_t *attrp);
int pthread_getattr_default_np(pthread_attr_t *attrp);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_setattr_default_np(
    attrp: *const pthread_attr_t,
) -> core::ffi::c_int;

extern "C" fn pthread_getattr_default_np(
    attrp: *mut pthread_attr_t,
) -> core::ffi::c_int;
```

---

## 意图

GNU 扩展函数（`_np` 后缀表示 Non-Portable）。提供获取和设置进程全局默认线程属性的功能。

- **`pthread_setattr_default_np`**：设置进程全局的默认线程属性。仅允许修改栈大小和守护页大小，其他属性字段必须为零。新值被 clamp 到上限以下，且全局默认值只能增大不能减小（单调不减）。
- **`pthread_getattr_default_np`**：获取进程全局的默认线程属性。返回一个仅设置了栈大小和守护页大小的属性对象，其余字段为零。

---

## 函数规约

### 1. pthread_setattr_default_np

#### 前置条件

- `attrp` 为非空指针（`!attrp.is_null()`），指向有效的 `pthread_attr_t`
- `attrp` 中除 `_a_stacksize` 和 `_a_guardsize` 外的所有字段必须为零（否则拒绝）

#### 后置条件

- Case 1 属性对象仅含有效的栈/守护页大小：
  1. `stack` = `min(attrp._a_stacksize, DEFAULT_STACK_MAX)` — clamp 到 8MB
  2. `guard` = `min(attrp._a_guardsize, DEFAULT_GUARD_MAX)` — clamp 到 1MB
  3. `inhibit_ptc()` — 阻止 PTC，准备修改全局变量
  4. `set_default_stacksize(max(current_stacksize, stack))` — 单调不减
  5. `set_default_guardsize(max(current_guardsize, guard))` — 单调不减
  6. `release_ptc()` — 恢复 PTC
  7. 返回 `0`
- Case 2 属性对象含有其他非零字段：返回 `EINVAL`，全局默认值不变

#### 不变量

- `default_stacksize` 和 `default_guardsize` 单调不减
- 任何修改全局默认值的操作都在 PTC 锁保护下进行（通过 `inhibit_ptc`）
- 修改后的值始终 <= 各自最大值（`DEFAULT_STACK_MAX` / `DEFAULT_GUARD_MAX`）

#### 算法

```
pthread_setattr_default_np(attrp):
  1. // 复制 attrp 并清零允许修改的字段，检查其余字段是否为零
  2. let mut tmp = *attrp
  3. tmp._a_stacksize = 0
  4. tmp._a_guardsize = 0
  5. if tmp != ZERO_ATTR { return EINVAL }  // 存在其他非零字段
  6. let stack = min((*attrp)._a_stacksize, DEFAULT_STACK_MAX)
  7. let guard = min((*attrp)._a_guardsize, DEFAULT_GUARD_MAX)
  8. inhibit_ptc()
  9. set_default_stacksize(max(default_stacksize(), stack))
  10. set_default_guardsize(max(default_guardsize(), guard))
  11. release_ptc()
  12. return 0
```

Rust 实现中，步骤 2 的"复制并比较"可使用 `core::ptr::read` 读取属性对象，然后手动将 `_a_stacksize` 和 `_a_guardsize` 字段归零，与零值结构体进行逐字段或字节级别比较。由于 `pthread_attr_t` 的大小固定，也可使用 `core::slice::from_raw_parts` 进行字节级比较。

---

### 2. pthread_getattr_default_np

#### 前置条件

- `attrp` 为非空指针（`!attrp.is_null()`），指向可写入的 `pthread_attr_t`

#### 后置条件

- Case 1 始终成功：返回 `0`
- 操作步骤：
  1. `acquire_ptc()` — 获取 PTC 锁
  2. `*attrp` 初始化为零值，仅设置 `_a_stacksize` 和 `_a_guardsize`
  3. `release_ptc()` — 释放 PTC 锁
- 返回的 `attrp` 中：
  - `_a_stacksize` = `default_stacksize()`
  - `_a_guardsize` = `default_guardsize()`
  - 其余字段为零

#### 不变量

- 读取 `default_stacksize` / `default_guardsize` 在 PTC 锁保护下进行

#### 算法

```
pthread_getattr_default_np(attrp):
  1. acquire_ptc()
  2. *attrp = ZERO_ATTR  // 清零
  3. (*attrp)._a_stacksize = default_stacksize()
  4. (*attrp)._a_guardsize = default_guardsize()
  5. release_ptc()
  6. return 0
```

---

## 内部辅助宏/函数（模块私有，不导出）

### MIN / MAX

原 C spec 中使用的局部宏。在 Rust 中直接使用 `core::cmp::min` / `core::cmp::max` 替代，无需额外定义。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t                // 定义于 pthread_impl 模块
  acquire_ptc()                 // PTC 锁获取 (定义于 pthread_impl 模块)
  release_ptc()                 // PTC 锁释放 (定义于 pthread_impl 模块)
  inhibit_ptc()                 // PTC 锁 + 阻止线程创建 (定义于 pthread_impl 模块)
  default_stacksize()           // 读取默认栈大小 (定义于 default_attr 模块)
  default_guardsize()           // 读取默认守护页大小 (定义于 default_attr 模块)
  set_default_stacksize()       // 设置默认栈大小 (定义于 default_attr 模块)
  set_default_guardsize()       // 设置默认守护页大小 (定义于 default_attr 模块)
Predefined Macros/Constants:
  DEFAULT_STACK_MAX = 8388608   // 栈大小上限 (8MB)
  DEFAULT_GUARD_MAX = 1048576   // 守护页大小上限 (1MB)
  EINVAL                        // 定义于 <errno.h>
Internal Helper:
  core::cmp::min / core::cmp::max  // 替代 C 的 MIN/MAX 宏

[GUARANTEE]
Exported Interfaces:
  extern "C" fn pthread_setattr_default_np(attrp: *const pthread_attr_t) -> c_int;
  // GNU 扩展：设置进程全局默认线程属性（仅栈大小和守护页大小可修改）
  extern "C" fn pthread_getattr_default_np(attrp: *mut pthread_attr_t) -> c_int;
  // GNU 扩展：获取进程全局默认线程属性
Internal Interface:
  (无内部接口 — 内部使用 core::cmp::min/max 和模块内 PTC/属性访问器)
