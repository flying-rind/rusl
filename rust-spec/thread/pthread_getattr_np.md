# pthread_getattr_np — Rust 接口归约

## 原始 C 接口

```c
int pthread_getattr_np(pthread_t t, pthread_attr_t *a);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_getattr_np(
    t: pthread_t,
    a: *mut pthread_attr_t,
) -> core::ffi::c_int;
```

---

## 意图

GNU 扩展函数（`_np` 后缀表示 Non-Portable）。从已存在的线程对象中提取其实际属性写入 `a`。不同于 `pthread_attr_get*` 系列函数从初始化时设定的属性对象读取，本函数获取线程运行时的真实状态：
- 分离状态：从线程的 `detach_state` 推导
- 守护页大小：从线程结构体直接读取
- 栈信息：若线程使用自定义栈，直接复制；否则通过 `mremap` 探测系统分配栈的实际大小

## 前置条件

- `t` 为有效的线程句柄（`t != PTHREAD_NULL`），指向一个有效的线程
- `a` 为非空指针（`!a.is_null()`），指向可写入的 `pthread_attr_t`
- 线程 `t` 未被销毁（其内部结构体仍然有效）

## 后置条件

- Case 1 始终成功（返回 `0`）：
  1. `*a` 全部字段清零
  2. `(*a)._a_detach` = 若 `t.detach_state >= DT_DETACHED` 则为 `1`，否则为 `0`
  3. `(*a)._a_guardsize` = `t.guard_size`
  4. **Case 1a** 线程有自定义栈（`t.stack != NULL`）：
     - `(*a)._a_stackaddr` = `t.stack`（栈顶地址）
     - `(*a)._a_stacksize` = `t.stack_size`
  5. **Case 1b** 线程使用系统分配栈（`t.stack == NULL`，主线程）：
     - 从 `libc.auxv` 计算主线程栈区域的近似地址
     - 使用 `mremap` 从低地址向高地址逐页探测，直到不再返回 `ENOMEM`
     - `(*a)._a_stackaddr` = 主线程栈区顶部（`libc.auxv` 页对齐）
     - `(*a)._a_stacksize` = 探测到的总页数累积大小

## 不变量

- 该函数不修改线程 `t` 的任何状态
- 对于主线程（`stack == NULL` 的情况），栈大小通过 `mremap` 试探性探测，该探测不会实际改变内存映射

## 算法

```
pthread_getattr_np(t, a):
  1. *a = ZERO_ATTR  // 清零所有字段
  2. (*a)._a_detach = if t.detach_state >= DT_DETACHED { 1 } else { 0 }
  3. (*a)._a_guardsize = t.guard_size
  4. if t.stack != NULL:
       // 自定义栈：直接复制
       (*a)._a_stackaddr = t.stack as usize
       (*a)._a_stacksize = t.stack_size
     else:
       // 系统分配的主线程栈：需要探测
       let p = (libc.auxv as *const u8 as usize) & !(PAGE_SIZE - 1)  // 页对齐
       let mut l = PAGE_SIZE
       (*a)._a_stackaddr = p
       loop:
           // 尝试 mremap(p - l - PAGE_SIZE, PAGE_SIZE, 2*PAGE_SIZE, 0)
           if mremap(ptr, PAGE_SIZE, 2*PAGE_SIZE, 0) == MAP_FAILED && errno == ENOMEM:
               l += PAGE_SIZE  // 向低地址扩展
           else:
               break
       (*a)._a_stacksize = l
  5. return 0
```

**栈探测机制说明**：
`mremap` 使用 `MREMAP_MAYMOVE=0`（第 4 参数为 0），因此不会实际移动映射。当尝试扩展的地址范围不可用时，返回 `MAP_FAILED` 且 `errno == ENOMEM`。循环从栈顶向低地址逐页尝试扩展，一旦不再返回 `ENOMEM`（可能成功或返回其他错误），即停止探测，此时累计的页数即为栈的近似大小。

在 Rust 实现中，`mremap` 调用需要通过 `extern "C"` FFI 或直接使用 `syscall` 指令。`errno` 的读取在 Rust 的 `no_std` 环境下需要通过 `__errno_location()` FFI 调用或直接从线程局部存储读取。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_attr_t           // 定义于 pthread_impl 模块
  pthread_t                // 线程句柄类型 (定义于 pthread_impl 模块)
  struct pthread           // 内部线程结构体 (定义于 pthread_impl 模块)
  libc                     // 全局 __libc 实例 (定义于 libc 模块)，提供 auxv 和 page_size
  mremap()                 // Linux mremap 系统调用 (通过 syscall 或 extern "C" FFI)
  __errno_location()       // 线程局部 errno 地址 (定义于 errno 模块)
Predefined Macros/Constants:
  DT_DETACHED              // 枚举值 (定义于 pthread_impl 模块)
  PAGE_SIZE                // = libc.page_size (定义于 libc 模块)
  MAP_FAILED               // 定义于 <sys/mman.h>
  ENOMEM                   // 定义于 <errno.h>
  PTHREAD_NULL             // 空线程句柄

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_getattr_np(t: pthread_t, a: *mut pthread_attr_t) -> c_int;
  // GNU 扩展：从线程 t 提取运行时实际属性写入 a，始终返回 0
Internal Interface:
  (无内部接口)
