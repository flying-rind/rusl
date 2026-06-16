# __unmapself.c 规约

> musl libc 内部"自我解除映射"函数。当线程需要释放自身栈空间时，无法在自身栈上执行 `munmap` 系统调用（因为会解除当前正在使用的栈）。此函数通过切换到共享的辅助栈并利用 `CRTJMP`（架构相关的尾调用跳转宏）在执行 `munmap` 和 `exit` 时不使用原始栈。

---

## 依赖图

```
__unmapself
  ├─> unmap_base, unmap_size             (本文件 static 变量)
  ├─> shared_stack[256]                   (本文件 static 辅助栈)
  └─> CRTJMP(do_unmap, stack)             (see arch/<arch>/reloc.h — 架构相关跳转)

do_unmap (static)
  ├─> __syscall(SYS_munmap, unmap_base, unmap_size)  (see syscall.h)
  └─> __syscall(SYS_exit)                             (see syscall.h)
```

---

## 全局变量

### unmap_base / unmap_size

```c
static void *unmap_base;
static size_t unmap_size;
```

[Visibility]: Internal (不导出) — 文件作用域静态变量

存储待解除映射的内存区域基址和大小。由 `__unmapself` 设置，由 `do_unmap` 消费。

### shared_stack

```c
static char shared_stack[256];
```

[Visibility]: Internal (不导出) — 文件作用域静态变量

256 字节的共享辅助栈。`__unmapself` 在执行 `munmap`/`exit` 前将栈指针切换到此辅助栈。因为所有使用 `__unmapself` 的线程在执行 munmap 后都立刻 exit，共享此栈是安全的（不存在并发使用）。

---

## 函数规约

### 1. do_unmap (static)

```c
static void do_unmap(void);
```

[Visibility]: Internal (不导出) — 文件作用域静态函数

#### Intent

在辅助栈上执行实际的 `munmap` 和 `exit` 操作。此函数通过 `CRTJMP` 被跳转到（不是通过常规调用），因此它不返回到调用者——执行 `munmap` 解除线程自身栈的映射后，通过 `exit` 系统调用终止线程。

#### 前置条件

- `unmap_base` 和 `unmap_size` 已由 `__unmapself` 设置
- 当前在 `shared_stack` 辅助栈上执行（由 `CRTJMP` 保证）
- 要解除映射的区域不包括 `shared_stack` 自身

#### 后置条件

- 线程不复存在：`munmap` 解除映射后，`exit` 系统调用终止线程
- 此函数永不返回

#### 系统算法

```
do_unmap():
  1. __syscall(SYS_munmap, unmap_base, unmap_size)   // 解除映射
  2. __syscall(SYS_exit)                               // 退出线程
```

#### 依赖

- `__syscall(SYS_munmap, ...)` — munmap 系统调用（见 `syscall.h`）
- `__syscall(SYS_exit)` — exit 系统调用（见 `syscall.h`）

---

### 2. __unmapself

```c
void __unmapself(void *base, size_t size);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用

#### Intent

安全地解除当前线程自身栈空间的映射。由于不能在使用中的栈上执行 `munmap`，此函数通过以下步骤解决：
1. 将待解除映射的区域记录到静态变量中
2. 计算 `shared_stack` 的 16 字节对齐栈顶
3. 使用 `CRTJMP` 宏切换到辅助栈并跳转到 `do_unmap`

`CRTJMP` 是架构相关的宏（定义在各架构的 `reloc.h` 中），它设置新的栈指针并跳转到目标函数，不创建栈帧，从而避免在原始栈上留下返回地址。

#### 前置条件

- `base` 指向待解除映射的内存区域（通常为线程自身的栈）
- `size` 为待解除映射区域的大小
- 此函数的栈帧不在 `[base, base+size)` 范围内（`CRTJMP` 切换栈后才执行 `munmap`）

#### 后置条件

- 线程不复存在（通过 `do_unmap` → `exit` 终止）
- 此函数永不返回

#### 系统算法

```
__unmapself(base, size):
  1. stack = shared_stack + sizeof(shared_stack)    // 辅助栈顶部
  2. stack -= (uintptr_t)stack % 16                 // 16 字节对齐
  3. unmap_base = base                              // 记录待解除映射区域
  4. unmap_size = size
  5. CRTJMP(do_unmap, stack)                        // 切换栈并跳转到 do_unmap
```

#### 不变量

- `shared_stack` 仅作为临时中转栈使用，在 `do_unmap` 执行完毕后对应的线程不再存在，因此不存在竞态条件
- `CRTJMP` 保证跳转时栈帧不依赖原栈

#### 依赖

- `CRTJMP()` — 架构相关的无返回尾调用跳转宏（见 `arch/<arch>/reloc.h`）
- `__syscall(SYS_munmap, ...)` — 解除内存映射（由 `do_unmap` 调用）
- `__syscall(SYS_exit)` — 终止线程（由 `do_unmap` 调用）
