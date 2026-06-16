# pthread_setattr_default_np.c 规约

> musl libc GNU 扩展函数集合。提供获取和设置进程全局默认线程属性的功能。这是非标准 POSIX 函数（`_np` 后缀表示 Non-Portable），定义于 `_GNU_SOURCE`。

---

## 依赖图

```
pthread_setattr_default_np
  ├─> memcmp()             (string.h, 用于属性对象比较)
  ├─> __inhibit_ptc()      (PTC 锁, pthread_impl.h)
  ├─> __release_ptc()      (PTC 锁, pthread_impl.h)
  ├─> __default_stacksize  (全局变量, default_attr.c)
  └─> __default_guardsize  (全局变量, default_attr.c)

pthread_getattr_default_np
  ├─> __acquire_ptc()      (PTC 锁, pthread_impl.h)
  ├─> __release_ptc()      (PTC 锁, pthread_impl.h)
  ├─> __default_stacksize  (全局变量, default_attr.c)
  └─> __default_guardsize  (全局变量, default_attr.c)
```

---

## 内部宏规约

### MIN / MAX

```c
#define MIN(a,b) ((a)<(b) ? (a) : (b))
#define MAX(a,b) ((a)>(b) ? (a) : (b))
```

[Visibility]: Internal (不导出) — 文件内部的局部宏

#### Intent

标准的最小/最大值比较宏。`MIN` 取较小值，`MAX` 取较大值。用于将输入值 clamp 到允许范围内。

---

## 函数规约

### 1. pthread_setattr_default_np

```c
int pthread_setattr_default_np(const pthread_attr_t *attrp);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出，仅在 `_GNU_SOURCE` 定义时可见（GNU 扩展）

#### Intent

设置进程全局的默认线程属性。**仅允许修改栈大小和守护页大小**，其他属性字段必须为零（表示不修改）。
- 新设定的栈/守护页大小被 clamp 到 `DEFAULT_STACK_MAX` (8MB) / `DEFAULT_GUARD_MAX` (1MB) 以下
- 全局默认值只能增大不能减小（取 MAX(current, new)）
- 修改在 PTC 锁的 `__inhibit_ptc` / `__release_ptc` 保护下进行，防止与线程创建的竞争条件

#### 前置条件

- `attrp != NULL`，指向有效的 `pthread_attr_t`
- `attrp` 中除 `_a_stacksize` 和 `_a_guardsize` 外的所有字段必须为零（否则拒绝）

#### 后置条件

- Case 1 属性对象仅含有效的栈/守护页大小：
  1. `stack = MIN(attrp->_a_stacksize, DEFAULT_STACK_MAX)` — clamp 到 8MB
  2. `guard = MIN(attrp->_a_guardsize, DEFAULT_GUARD_MAX)` — clamp 到 1MB
  3. `__inhibit_ptc()` — 阻止 PTC，准备修改全局变量
  4. `__default_stacksize = MAX(__default_stacksize, stack)` — 单调不减
  5. `__default_guardsize = MAX(__default_guardsize, guard)` — 单调不减
  6. `__release_ptc()` — 恢复 PTC
  7. 返回 `0`
- Case 2 属性对象含有其他非零字段：返回 `EINVAL`，全局默认值不变

#### 不变量

- `__default_stacksize` 和 `__default_guardsize` 单调不减
- 任何修改全局默认值的操作都在 PTC 锁保护下进行
- 修改后的值始终 <= 各自最大值（DEFAULT_STACK_MAX / DEFAULT_GUARD_MAX）

#### 系统算法

```
pthread_setattr_default_np(attrp):
  1. tmp = *attrp, zero = {0}
  2. tmp._a_stacksize = 0; tmp._a_guardsize = 0  // 清零允许的字段
  3. if (memcmp(&tmp, &zero, sizeof tmp)) return EINVAL  // 检查其他字段是否为 0
  4. stack = MIN(attrp->_a_stacksize, DEFAULT_STACK_MAX)
  5. guard = MIN(attrp->_a_guardsize, DEFAULT_GUARD_MAX)
  6. __inhibit_ptc()
  7. __default_stacksize = MAX(__default_stacksize, stack)
  8. __default_guardsize = MAX(__default_guardsize, guard)
  9. __release_ptc()
  10. return 0
```

**设计说明**：
- 函数拒绝修改除栈/守护页以外的任何属性，因为这些属性在进程层面没有合理的"全局默认值"语义
- 使用 `__inhibit_ptc` 而非 `__acquire_ptc`，因为修改全局默认值需要阻止线程创建（而不仅仅是读取）
- 单调不减策略防止后续调用者意外降低已设定的默认值

#### 依赖

- `memcmp()` — 内存比较函数（`<string.h>`）
- `__inhibit_ptc()` — PTC 锁定 + 阻止（定义于 thread 模块内部）
- `__release_ptc()` — PTC 解锁（定义于 thread 模块内部）
- `__default_stacksize` — 全局变量（定义于 `default_attr.c`）
- `__default_guardsize` — 全局变量（定义于 `default_attr.c`）
- `DEFAULT_STACK_MAX` — 宏，定义于 `pthread_impl.h`，值为 `8<<20` (8MB)
- `DEFAULT_GUARD_MAX` — 宏，定义于 `pthread_impl.h`，值为 `1<<20` (1MB)

---

### 2. pthread_getattr_default_np

```c
int pthread_getattr_default_np(pthread_attr_t *attrp);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出，仅在 `_GNU_SOURCE` 定义时可见（GNU 扩展）

#### Intent

获取进程全局的默认线程属性。返回一个仅设置了栈大小和守护页大小的属性对象，其余字段为零。在 PTC 锁保护下读取全局默认值。

#### 前置条件

- `attrp != NULL`，指向可写入的 `pthread_attr_t`

#### 后置条件

- Case 1 始终成功：返回 `0`
- 操作步骤：
  1. `__acquire_ptc()` — 获取 PTC 锁
  2. `*attrp` 初始化为复合字面量，仅设置 `_a_stacksize` 和 `_a_guardsize`
  3. `__release_ptc()` — 释放 PTC 锁
- 返回的 `attrp` 中：
  - `_a_stacksize` = `__default_stacksize`
  - `_a_guardsize` = `__default_guardsize`
  - 其余字段为零

#### 不变量

- 读取 `__default_stacksize` / `__default_guardsize` 在 PTC 锁保护下进行

#### 系统算法

```
pthread_getattr_default_np(attrp):
  1. __acquire_ptc()
  2. *attrp = (pthread_attr_t){ ._a_stacksize = __default_stacksize,
                                ._a_guardsize = __default_guardsize }
  3. __release_ptc()
  4. return 0
```

#### 依赖

- `__acquire_ptc()` — PTC 锁获取（定义于 thread 模块内部）
- `__release_ptc()` — PTC 锁释放（定义于 thread 模块内部）
- `__default_stacksize` — 全局变量（定义于 `default_attr.c`）
- `__default_guardsize` — 全局变量（定义于 `default_attr.c`）
- `pthread_impl.h` — 内部头文件，定义 `_a_stacksize` / `_a_guardsize` 宏
