# pthread_getattr_np.c 规约

> musl libc GNU 扩展函数。从已存在的线程对象中获取其实际属性（如 detach 状态、栈地址、栈大小、守护页大小）。当线程使用系统分配的栈时，通过 `mremap` 探测栈的实际大小。

---

## 依赖图

```
pthread_getattr_np
  ├─> libc.auxv          (libc 全局上下文, libc.h)
  ├─> PAGE_SIZE          (宏 = libc.page_size, libc.h)
  ├─> mremap()           (sys/mman.h, Linux 系统调用)
  ├─> struct pthread     (内部线程结构体, pthread_impl.h)
  └─> DT_DETACHED        (枚举值, pthread_impl.h)
```

---

## 函数规约

### 1. pthread_getattr_np

```c
int pthread_getattr_np(pthread_t t, pthread_attr_t *a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出，仅在 `_GNU_SOURCE` 定义时可见（GNU 扩展）

#### Intent

从已存在的线程 `t` 中提取其实际属性写入 `a`。不同于 `pthread_attr_get*` 系列函数从初始化时设定的属性对象读取，本函数获取线程运行时的真实状态：
- 分离状态：从 `detach_state` 推导
- 守护页大小：从线程结构体直接读取
- 栈信息：若线程使用自定义栈，直接复制；否则通过探测获取系统分配栈的实际大小

#### 前置条件

- `t != NULL` 且 `t != PTHREAD_NULL`，指向一个有效的线程
- `a != NULL`，指向可写入的 `pthread_attr_t`
- 线程 `t` 未被销毁（其 `pthread` 结构体仍然有效）

#### 后置条件

- Case 1 始终成功（返回 `0`）：
  1. `*a` 全部字段清零
  2. `a->_a_detach` = (`t->detach_state >= DT_DETACHED`) ? 1 : 0
  3. `a->_a_guardsize` = `t->guard_size`
  4. **Case 1a** 线程有自定义栈（`t->stack != NULL`）：
     - `a->_a_stackaddr` = `t->stack`（栈顶地址）
     - `a->_a_stacksize` = `t->stack_size`
  5. **Case 1b** 线程使用系统分配栈（`t->stack == NULL`）：
     - 从 `libc.auxv` 计算主线程栈区域的近似地址
     - 使用 `mremap` 从低地址向高地址逐页探测，直到不再返回 `ENOMEM`
     - `a->_a_stackaddr` = 主线程栈区顶部（`libc.auxv` 页对齐）
     - `a->_a_stacksize` = 探测到的总页数累积大小

#### 不变量

- 该函数不修改线程 `t` 的任何状态
- 对于主线程（`stack == NULL` 的情况），栈大小通过 `mremap` 试探性探测，该探测不会实际改变内存映射

#### 系统算法

```
pthread_getattr_np(t, a):
  1. *a = (pthread_attr_t){0}          // 清零所有字段
  2. a->_a_detach = t->detach_state >= DT_DETACHED  // 计算是否 detached
  3. a->_a_guardsize = t->guard_size
  4. if (t->stack != NULL):
       // 自定义栈：直接复制
       a->_a_stackaddr = (uintptr_t)t->stack
       a->_a_stacksize = t->stack_size
     else:
       // 系统分配的主线程栈：需要探测
       p = (char *)libc.auxv                 // auxv 位于栈顶
       l = PAGE_SIZE                          // 从 1 页开始探测
       p += -(uintptr_t)p & PAGE_SIZE-1       // 页对齐（向下）
       a->_a_stackaddr = (uintptr_t)p
       while (mremap(p-l-PAGE_SIZE, PAGE_SIZE, 2*PAGE_SIZE, 0) == MAP_FAILED
              && errno == ENOMEM):
           l += PAGE_SIZE                     // 向低地址扩展探测
       a->_a_stacksize = l
  5. return 0
```

**设计说明**：

- **栈探测机制**：当线程使用系统分配栈（主线程）时，musl 没有记录栈大小。本函数通过 `mremap` 尝试向低地址扩展当前栈顶页的映射来"探测"栈的实际大小。`mremap` 调用使用 `MREMAP_MAYMOVE=0`（第 4 参数为 0），因此失败时返回 `MAP_FAILED` 且 `errno == ENOMEM` 表示当前大小即为实际栈大小。

- **`libc.auxv`** 是辅助向量数组，位于进程地址空间的高地址端（栈的上方）。musl 使用它来定位主线程栈的顶端。

- 探测循环从栈顶向低地址方向逐页尝试扩展，一旦 `mremap` 不再返回 `ENOMEM`（可能成功或返回其他错误），就停止循环，此时 `l` 累计的总大小即为栈的近似大小。

#### 依赖

- `libc` — 全局 `struct __libc` 实例（定义于 `libc.h`），本函数使用 `libc.auxv` 定位主线程栈
- `PAGE_SIZE` — 宏，展开为 `libc.page_size`（定义于 `libc.h`）
- `mremap()` — Linux 内存重映射系统调用（声明于 `<sys/mman.h>`）
- `MAP_FAILED` — 宏，定义于 `<sys/mman.h>`
- `errno` — 线程局部错误号（`<errno.h>`）
- `ENOMEM` — 宏，定义于 `<errno.h>`
- `struct pthread` — 内部线程结构体（定义于 `pthread_impl.h`）
- `DT_DETACHED` — 枚举值，来自 `pthread_impl.h` 中的 `enum` 定义
- `pthread_attr_t` — 线程属性类型
- `pthread_impl.h` — 内部头文件，定义 `_a_detach` / `_a_guardsize` / `_a_stackaddr` / `_a_stacksize` 宏
