# __unmapself — Rust 接口归约

> rusl 内部"自我解除映射"函数。当线程需要释放自身栈空间时，无法在自身栈上执行 `munmap` 系统调用（因为会解除当前正在使用的栈）。此函数通过切换到共享的辅助栈并执行 `munmap` 和 `exit` 系统调用来安全终止线程。

## 原始 C 接口

```c
void __unmapself(void *base, size_t size);
```

[Visibility]: Internal — 被 `pthread_impl.h` 声明为 hidden，仅 musl/rusl 内部使用

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __unmapself(base: *mut core::ffi::c_void, size: usize) -> !;
```

> 注意：返回类型为 `!`（never type），表示此函数永不返回。

---

## 依赖图

```
__unmapself
  ├─> unmap_base, unmap_size             (本模块 static 变量)
  ├─> shared_stack                        (本模块 static 辅助栈)
  └─> CRTJMP(do_unmap, stack)             (架构相关的尾调用跳转)

do_unmap (内部 static)
  ├─> linux syscall: SYS_munmap           (解除映射)
  └─> linux syscall: SYS_exit             (退出线程)
```

---

## 内部数据结构（不对外导出）

### 辅助栈

```rust
// 256 字节的共享辅助栈。因为使用 unmapself 的线程都立刻 exit，
// 共享此栈是安全的（不存在并发使用）
static mut SHARED_STACK: [u8; 256] = [0u8; 256];
```

### 解除映射参数

```rust
// 存储待解除映射的内存区域基址和大小
static mut UNMAP_BASE: *mut core::ffi::c_void = core::ptr::null_mut();
static mut UNMAP_SIZE: usize = 0;
```

---

## 函数规约

### 1. do_unmap (内部 static, 不对外导出)

```rust
// 文件作用域静态函数，通过 CRTJMP 跳转调用，永不返回
extern "C" fn do_unmap() -> !;
```

#### Intent

在辅助栈上执行实际的 `munmap` 和 `exit` 操作。此函数通过架构相关的尾调用跳转机制被调用（不是通过常规 `call` 指令），因此它不返回到调用者 -- 执行 `munmap` 解除线程自身栈的映射后，通过 `exit` 系统调用终止线程。

#### 前置条件

- `UNMAP_BASE` 和 `UNMAP_SIZE` 已由 `__unmapself` 设置
- 当前在 `SHARED_STACK` 辅助栈上执行
- 要解除映射的区域不包括 `SHARED_STACK` 自身

#### 后置条件

- 线程不复存在：`munmap` 解除映射后，`exit` 系统调用终止线程
- 此函数永不返回

#### 系统算法

```
do_unmap() -> !:
  1. syscall(SYS_munmap, UNMAP_BASE, UNMAP_SIZE)
  2. syscall(SYS_exit, 0)
  3. unreachable!()
```

---

### 2. __unmapself

```rust
pub extern "C" fn __unmapself(base: *mut core::ffi::c_void, size: usize) -> !;
```

#### Intent

安全地解除当前线程自身栈空间的映射。由于不能在使用中的栈上执行 `munmap`，此函数通过以下步骤解决：
1. 将待解除映射的区域记录到静态变量中
2. 计算 `SHARED_STACK` 的 16 字节对齐栈顶
3. 切换到辅助栈并跳转到 `do_unmap`

在 Rust 中，由于 Rust 不直接支持 `CRTJMP`（架构相关的尾调用跳转宏），此函数需要使用 `global_asm!` 内联汇编实现栈切换逻辑。纯 Rust 部分负责设置静态变量。

#### 前置条件

- `base` 指向待解除映射的内存区域（通常为线程自身的栈）
- `size` 为待解除映射区域的大小
- 此函数的栈帧不在 `[base, base+size)` 范围内

#### 后置条件

- 线程不复存在（通过 `do_unmap` -> `exit` 终止）
- 此函数永不返回

#### 系统算法

```
__unmapself(base, size) -> !:
  1. // 纯 Rust 部分：设置静态变量
  2. UNMAP_BASE = base
  3. UNMAP_SIZE = size
  4. // 内联汇编部分：切换栈并跳转
  5. unsafe {
  6.   global_asm!(切换栈指针到 SHARED_STACK 顶部, 跳转到 do_unmap)
  7. }
  8. unreachable!()
```

> Rust 实现的挑战：`CRTJMP` 需要直接操作栈指针寄存器 (`rsp`/`sp`) 并执行尾调用跳转，这必须通过内联汇编实现。`extern "C" fn __unmapself` 在设置参数后，调用架构相关的汇编辅助宏。

#### 不变量

- `SHARED_STACK` 仅作为临时中转栈使用，在 `do_unmap` 执行完毕后对应的线程不再存在，因此不存在竞态条件
- 栈切换必须通过汇编保证不依赖原栈帧

---

## Rust 内部实现注意事项

由于 `__unmapself` 涉及栈切换和永不返回，Rust 实现需要：

1. 使用 `global_asm!` 或独立的 `.s` 汇编文件实现 `CRTJMP` 栈切换逻辑
2. `__unmapself` 的 Rust 部分设置静态变量后，调用汇编辅助函数进行栈切换
3. `do_unmap` 必须是 `extern "C"` 可见（供汇编跳转），且标记为 `-> !`（发散函数）

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  global_asm! / arch-specific .s files   // 依赖1: 架构相关汇编（栈切换 CRTJMP）
  linux syscall: SYS_munmap               // 依赖2: 解除内存映射
  linux syscall: SYS_exit                 // 依赖3: 终止线程

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __unmapself(base: *mut c_void, size: usize) -> !;
                                         // 本模块保证对外提供与 C ABI 兼容的 __unmapself 符号
Internal Interface:
  extern "C" fn do_unmap() -> !;         // 供汇编代码调用的内部函数
                                         // 静态变量 SHARED_STACK, UNMAP_BASE, UNMAP_SIZE
