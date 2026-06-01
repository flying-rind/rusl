# lite_malloc -- Rust 接口归约

> 轻量级 bump 分配器，用于 rusl 早期初始化阶段以及作为 `malloc` 的弱符号默认实现。
> 当完整的 malloc 实现（mallocng / oldmalloc）初始化后，会通过 `__libc_malloc_impl` 的强符号
> 覆盖本文件提供的弱符号，从而替换掉简易分配逻辑。
>
> **Rust 设计原则**: 对外导出符号保持 ABI 兼容（`extern "C"`），内部实现使用安全 Rust 抽象
> 重新设计。禁止使用 `libc` crate，所有 syscall 通过 `asm!` 直接发起。

---

## 依赖图

```
malloc (weak, Public)
  └── default_malloc (内部)
        └── __libc_malloc_impl (weak alias of __simple_malloc)
              └── __simple_malloc (内部 bump 分配器)
                    ├── check_stack_collision (内部) -- 栈冲突检测
                    │     └── AUXV (全局 AtomicPtr, 由 crt/init 设置)
                    ├── sys_brk (内部 syscall 封装)
                    │     └── asm!("syscall") -- 发起 SYS_brk
                    ├── sys_mmap (内部 syscall 封装)
                    │     └── asm!("syscall") -- 发起 SYS_mmap
                    ├── BUMP_LOCK (内部 AtomicI32 自旋锁)
                    ├── PAGE_SIZE (全局 AtomicU32, 由 crt/init 设置)
                    ├── BUMP_BRK (内部 AtomicUsize -- 当前 brk 值)
                    ├── BUMP_CUR (内部 AtomicUsize -- 当前分配游标)
                    ├── BUMP_END (内部 AtomicUsize -- 当前区域末尾)
                    └── BUMP_MMAP_STEP (内部 AtomicU8 -- mmap 几何增长步数)

__libc_malloc (Internal, exported)
  └── __libc_malloc_impl (同上)

__bump_lockptr (Internal, exported)
  └── BUMP_LOCK (内部 AtomicI32)
```

---

## 模块划分

Rust 实现采用子模块组织，提升内聚性：

```
src/malloc/lite_malloc/
├── mod.rs              -- 模块入口，导出所有 extern "C" 符号；定义内部全局静态变量
├── bump.rs             -- __simple_malloc bump 分配器核心逻辑
├── stack_check.rs      -- check_stack_collision 栈冲突检测
└── syscalls.rs         -- sys_brk / sys_mmap 原始系统调用封装
```

---

## 一、内部常量

### ALIGN（最小对齐常量）

```rust
/// bump 分配器的最小对齐粒度。所有分配地址按不大于此值的 2 的幂向上对齐。
const ALIGN: usize = 16;
```

[Visibility]: Internal -- `pub(crate)` 模块级常量

- **值**: 16 字节
- **语义**: 满足 x86_64 等架构上 `long double`、`__int128` 等类型的对齐需求

### STACK_ESTIMATE（栈区域深度估计）

```rust
/// 用于栈冲突检测的栈区域深度启发式估计值（8MB）
const STACK_ESTIMATE: usize = 8 << 20;
```

[Visibility]: Internal -- `pub(crate)` 模块级常量

### MMAP_STEP_MAX（mmap 几何增长上限）

```rust
/// mmap_step 的最大值，对应 PAGE_SIZE << 6 = 64 * PAGE_SIZE
const MMAP_STEP_MAX: u8 = 12;
```

[Visibility]: Internal -- `pub(crate)` 模块级常量

### WASITE_THRESHOLD（浪费比例阈值分母）

```rust
/// 浪费比例阈值：当 (req - n) > req / 8 时触发独立 mmap 区域策略
const WASTE_THRESHOLD_DENOM: usize = 8;
```

[Visibility]: Internal -- `pub(crate)` 模块级常量

### 系统调用号常量

```rust
/// SYS_brk 系统调用号（按架构定义，示例为 x86_64）
const SYS_BRK: isize = 12;  // x86_64: 12
/// SYS_mmap 系统调用号（按架构定义，示例为 x86_64）
const SYS_MMAP: isize = 9;  // x86_64: 9
```

[Visibility]: Internal -- `pub(crate)` 模块级常量；实际值由 `#[cfg(target_arch = "...")]` 条件编译确定

### ENOMEM 错误码

```rust
/// 内存不足 errno 值
const ENOMEM: i32 = 12;
```

[Visibility]: Internal -- `pub(crate)` 模块级常量

### mmap 相关常量

```rust
/// mmap 失败返回值
const MAP_FAILED: usize = !0usize;
/// 内存保护: 可读
const PROT_READ: i32  = 0x1;
/// 内存保护: 可写
const PROT_WRITE: i32 = 0x2;
/// 映射标志: 私有匿名
const MAP_PRIVATE: i32  = 0x02;
/// 映射标志: 匿名映射
const MAP_ANONYMOUS: i32 = 0x20;
```

[Visibility]: Internal -- `pub(crate)` 模块级常量

### 对冲量（Over-allocate Margin）

```rust
/// 大块分配的额外对冲量（用于减少碎片），单位字节
const OVER_MARGIN: usize = 4096;
```

[Visibility]: Internal -- `pub(crate)` 模块级常量

---

## 二、内部全局状态

### BUMP_LOCK（分配器自旋锁）

```rust
/// bump 分配器的互斥自旋锁，保护所有静态分配状态
static BUMP_LOCK: AtomicI32 = AtomicI32::new(0);
```

[Visibility]: Internal -- `pub(crate)` 模块级静态变量

- **类型**: `core::sync::atomic::AtomicI32`
- **语义**: 保护 `__simple_malloc` 内部静态变量的互斥锁
- **访问规则**: 通过 `BUMP_LOCK_ACQUIRE()` / `BUMP_LOCK_RELEASE()` 内联函数操作
- **不变量**: 值为 0 时表示无竞争，非 0 时表示已被持有

### BUMP_LOCK_ACQUIRE / BUMP_LOCK_RELEASE（锁操作内联函数）

```rust
/// 获取 bump 分配器自旋锁（忙等待）
#[inline]
fn bump_lock_acquire() {
    while BUMP_LOCK.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
}

/// 释放 bump 分配器自旋锁
#[inline]
fn bump_lock_release() {
    BUMP_LOCK.store(0, Ordering::Release);
}
```

[Visibility]: Internal -- `pub(crate)` 模块级内联函数

Rust 设计: 用 `core::sync::atomic::AtomicI32` + `core::hint::spin_loop()` 替换 C 的 `LOCK(lock)` / `UNLOCK(lock)` 宏和 `__lock` / `__unlock` 依赖。

### __bump_lockptr（对外暴露的锁指针）

```rust
/// fork 安全机制所需的锁指针
/// 在 C ABI 中导出为 `volatile int *const __bump_lockptr`
#[no_mangle]
static __bump_lockptr: &AtomicI32 = &BUMP_LOCK;
```

[Visibility]: Internal (不导出给用户) -- musl 内部符号，由 fork 处理器引用

- **类型**: `&'static AtomicI32`
- **语义**: fork 安全机制所需的锁指针，fork 前由 `__malloc_atfork` 加锁，`fork()` 返回后在子进程中 `__post_Fork` 解锁
- **不变量**: 始终指向 `BUMP_LOCK`

**注意**: Rust 中无法直接导出 `volatile int *const` 类型的全局 C 变量。实际实现需要在 `mod.rs` 中定义一个 `#[no_mangle] static mut __bump_lockptr: *mut c_int`，并在模块初始化时将 `&raw const BUMP_LOCK` 转换为其地址。由于 `AtomicI32` 内部表示为 `i32`，可安全通过 `*(&BUMP_LOCK as *const AtomicI32 as *const c_int)` 获取指针。

### BUMP_BRK / BUMP_CUR / BUMP_END（分配器内部游标）

```rust
/// 当前 brk 值（数据段末尾），页对齐
static BUMP_BRK: AtomicUsize = AtomicUsize::new(0);
/// 当前分配游标（下次分配的起始地址）
static BUMP_CUR: AtomicUsize = AtomicUsize::new(0);
/// 当前分配区域末尾（不可分配的边界地址）
static BUMP_END: AtomicUsize = AtomicUsize::new(0);
```

[Visibility]: Internal -- `pub(crate)` 模块级静态变量

- **不变量**（在持有 `BUMP_LOCK` 时）: `BUMP_BRK <= BUMP_CUR <= BUMP_END`
- **注意**: 虽然使用 `AtomicUsize` 类型，但这些变量在持有锁时才被修改，不需要原子操作（仅用于类型系统方便和跨线程可见性）

### BUMP_MMAP_STEP（mmap 几何增长步数）

```rust
/// mmap 区域几何增长步数（0..=12），控制新区域最小尺寸
static BUMP_MMAP_STEP: AtomicU8 = AtomicU8::new(0);
```

[Visibility]: Internal -- `pub(crate)` 模块级静态变量

### PAGE_SIZE（运行时页面大小）

```rust
/// 运行时页面大小，由 crt/init 在初始化时设置
static PAGE_SIZE: AtomicU32 = AtomicU32::new(4096); // 默认 4096
```

[Visibility]: Internal -- `pub(crate)` 模块级静态变量。对应 C 的 `libc.page_size`

### AUXV（内核辅助向量指针）

```rust
/// 内核辅助向量指针，由 crt/init 在初始化时设置
/// 用于栈冲突检测中的主线程栈区域推断
static AUXV: AtomicPtr<c_ulong> = AtomicPtr::new(core::ptr::null_mut());
```

[Visibility]: Internal -- `pub(crate)` 模块级静态变量。对应 C 的 `libc.auxv`

---

## 三、内部 syscall 封装 (syscalls.rs)

### sys_brk -- brk 系统调用

```rust
/// 发起 brk 系统调用，扩展/获取数据段末尾地址
///
/// - 参数 `addr == 0`: 返回当前 brk 值
/// - 参数 `addr != 0`: 尝试设置 brk 为 addr，返回新的 brk 值（失败时返回旧值）
#[inline]
unsafe fn sys_brk(addr: usize) -> usize;
```

[Visibility]: Internal -- `pub(crate)` 模块级函数

#### 前置条件
- 内核已完成初始化，系统调用机制可用

#### 后置条件
- 返回当前 brk 值
- 若设置失败（如请求的地址不可用），返回旧 brk 值

#### 实现策略
通过 `core::arch::asm!` 内联汇编直接发起 `syscall` 指令（x86_64）或等价的架构相关指令：

```rust
// x86_64 示例
unsafe fn sys_brk(addr: usize) -> usize {
    let ret: usize;
    core::arch::asm!(
        "syscall",
        inlateout("rax") SYS_BRK => ret,
        in("rdi") addr,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret
}
```

#### 依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `SYS_BRK` | 本模块常量 | brk 系统调用号（按架构条件编译） |

---

### sys_mmap -- mmap 系统调用

```rust
/// 发起 mmap 系统调用，创建内存映射
///
/// 封装 Linux mmap 系统调用，支持匿名映射用于动态内存分配。
#[inline]
unsafe fn sys_mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, offset: isize) -> *mut c_void;
```

[Visibility]: Internal -- `pub(crate)` 模块级函数

#### 前置条件
- 内核已完成初始化
- `len > 0`

#### 后置条件
- **Case 1 — 成功**: 返回映射区域的起始地址（内核选择的虚拟地址）
- **Case 2 — 失败**: 返回 `MAP_FAILED`（即 `!0usize`），`errno` 由内核设置

#### 实现策略
通过 `core::arch::asm!` 内联汇编直接发起 `syscall` 指令：

```rust
// x86_64 示例
unsafe fn sys_mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, offset: isize) -> *mut c_void {
    let ret: usize;
    core::arch::asm!(
        "syscall",
        inlateout("rax") SYS_MMAP => ret,
        in("rdi") addr as usize,
        in("rsi") len,
        in("rdx") prot as usize,
        in("r10") flags as usize,
        in("r8") fd as usize,
        in("r9") offset,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags)
    );
    ret as *mut c_void
}
```

#### 依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `SYS_MMAP` | 本模块常量 | mmap 系统调用号（按架构条件编译） |
| `MAP_FAILED` | 本模块常量 | mmap 失败哨兵值 |

**设计要点**: rusl 不使用 `libc` crate 的 `mmap` 封装。对应 C 的 `__mmap`（`src/mman/mmap.c`）内部实现，rusl 直接为 lite_malloc 模块提供专用的 `sys_mmap` 封装，避免跨 crate 依赖。

---

## 四、内部辅助函数

### check_stack_collision -- 栈区间冲突检测 (stack_check.rs)

```rust
/// 检测 brk 扩展区间 `[old, new)` 是否会与线程栈区域发生交叉
///
/// 对应 C 的 `traverses_stack_p`，使用安全 Rust 重新设计。
fn check_stack_collision(old: usize, new: usize) -> bool;
```

[Visibility]: Internal -- `pub(crate)` 模块级函数

#### 意图
检测 `brk` 扩展区间 `[old, new)` 是否会与主线程栈或当前线程栈区域发生交叉，作为对有缺陷的 `brk` 实现（可能跨越栈区域的）的白名单防御。

#### 前置条件
- `old` 和 `new` 为有效的虚拟地址（`usize`），表示提议的堆扩展区间下界和上界
- `new >= old`（调用者保证）
- `AUXV` 已初始化（指向内核传递的辅助向量，非空）

#### 后置条件
- **Case 1 — 返回 true（冲突）**: 区间 `[old, new)` 与以下区域之一存在交集：
  - 区间 `[max(0, auxv_addr - STACK_ESTIMATE), auxv_addr)`（推测为主线程栈区域）
  - 区间 `[max(0, &stack_marker - STACK_ESTIMATE), &stack_marker)`（当前线程栈区域）
- **Case 2 — 返回 false（安全）**: 未检测到冲突

#### 算法
```rust
fn check_stack_collision(old: usize, new: usize) -> bool {
    // 检测 brk 扩展是否跨越主线程栈区域（以 AUXV 地址推断栈顶）
    let auxv = AUXV.load(Ordering::Relaxed) as usize;
    if auxv != 0 {
        let stack_top = auxv;
        let stack_bottom = stack_top.saturating_sub(STACK_ESTIMATE);
        if new > stack_bottom && old < stack_top {
            return true;
        }
    }

    // 检测 brk 扩展是否跨越当前线程栈区域
    let stack_marker: u8 = 0; // 栈帧标记变量
    let cur_stack_top = &stack_marker as *const u8 as usize;
    let cur_stack_bottom = cur_stack_top.saturating_sub(STACK_ESTIMATE);
    if new > cur_stack_bottom && old < cur_stack_top {
        return true;
    }

    false
}
```

#### 局限性
- `STACK_ESTIMATE`（8MB）是启发式常量，不等于实际的 `RLIMIT_STACK`；保守起见，若栈在 8MB 以下则可能漏报，但不会导致误杀
- 依赖 `AUXV` 恰好位于主线程栈"上方"的假设（Linux 内核将 auxv 放置在高地址区）

#### 依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `AUXV` | 本模块全局 AtomicPtr | 内核辅助向量地址，由 crt/init 设置 |
| `STACK_ESTIMATE` | 本模块常量 | 栈区域深度启发式估计（8MB） |

---

## 五、核心 bump 分配器 (bump.rs)

### __simple_malloc -- bump 分配器核心

```rust
/// bump 分配器的核心实现。
///
/// 优先通过 brk 扩展堆，在 brk 不可用或可能导致栈冲突时回退到 mmap。
/// 对于大块分配（浪费超过 1/8 时），采用几何增长的独立 mmap 区域减少碎片。
///
/// 对应 C 的 `__simple_malloc`，使用安全 Rust 抽象重新设计内部逻辑。
fn simple_malloc(n: usize) -> *mut c_void;
```

[Visibility]: Internal -- `pub(crate)` 模块级函数。通过 `weak_alias` 机制以弱符号 `__libc_malloc_impl` 对外暴露。

#### 意图
实现一个极简的 bump 分配器，用于 libc 早期初始化阶段以及作为完整 malloc 未链接时的 fallback。

#### 前置条件
- `n` 为请求分配的大小（字节）
- `BUMP_LOCK` 处于可获取状态（无死锁风险）
- 系统调用 `sys_brk` 和 `sys_mmap` 可用（内核已初始化）

#### 后置条件

**Case 1 -- 参数非法 (`n > usize::MAX / 2`)**:
- 返回值: `core::ptr::null_mut()`
- `errno` 设置为 `ENOMEM`
- 堆状态不变

**Case 2 -- 分配成功（brk 路径）**:
- 返回值: 指向新分配内存的指针，地址按 `min(2^k, ALIGN)` 对齐
- 分配的内存位于数据段（heap），紧邻之前已分配的区域
- `BUMP_CUR` 自增 `n`，`BUMP_END` 可能因 `sys_brk` 扩展而增加
- 锁已释放

**Case 3 -- 分配成功（mmap 直接返回，小请求）**:
- `sys_mmap` 成功且 `new_area == false`
- 返回值: `sys_mmap` 返回的内存地址，页对齐
- 锁已释放
- 不修改 `BUMP_CUR`、`BUMP_END`、`BUMP_BRK`

**Case 4 -- 分配成功（mmap 新区域）**:
- `sys_mmap` 成功且 `new_area == true`
- `BUMP_CUR` 设为 mmap 返回地址，`BUMP_END` 设为 `BUMP_CUR + req`
- `BUMP_MMAP_STEP` 可能递增（最大到 12）
- 返回值: 从新 mmap 区域分配的指针，按 bump 逻辑对齐
- 锁已释放

**Case 5 -- 分配失败（mmap 失败）**:
- 返回值: `core::ptr::null_mut()`
- `errno` 由 sys_mmap 失败原因决定
- 锁已释放
- 堆状态不变

#### 系统算法

```
1. 参数校验
   if n > usize::MAX / 2:
       set_errno(ENOMEM); return null_mut()
   if n == 0: n = 1

2. 对齐计算（2 的幂指数增长，上限 ALIGN=16）
   align = 1
   while align < n && align < ALIGN:
       align <<= 1

3. 加锁 bump_lock_acquire()

4. 地址对齐
   cur = BUMP_CUR.load(Relaxed)
   cur = cur.wrapping_add(cur.wrapping_neg() & (align - 1))
   BUMP_CUR.store(cur, Relaxed)

5. 空间不足时的扩展逻辑
   end = BUMP_END.load(Relaxed)
   if n > end - cur:
       req = page_align(n - (end - cur))

       // 首次调用：获取初始 brk
       if cur == 0:
           brk = page_align(unsafe { sys_brk(0) })
           BUMP_BRK.store(brk, Relaxed)
           BUMP_CUR.store(brk, Relaxed)
           BUMP_END.store(brk, Relaxed)
           cur = brk; end = brk

       // 尝试 brk 扩展（优先路径）
       brk = BUMP_BRK.load(Relaxed)
       if brk == end && req < usize::MAX - brk
          && !check_stack_collision(brk, brk + req)
          && unsafe { sys_brk(brk + req) } == brk + req:
           BUMP_BRK.store(brk + req, Relaxed)
           BUMP_END.store(end + req, Relaxed)

       // 回退到 mmap（brk 失败或不可用）
       else:
           req = page_align(n)
           new_area = false

           // 启发式：浪费超过 1/8 时创建新区域
           if req - n > req / WASTE_THRESHOLD_DENOM:
               step = BUMP_MMAP_STEP.load(Relaxed)
               min_req = PAGE_SIZE.load(Relaxed) as usize << (step / 2)
               // 用新区域剩余更少 → 创建新区域
               if min_req - n > end - cur:
                   req = max(req, min_req)
                   if step < MMAP_STEP_MAX: BUMP_MMAP_STEP.store(step + 1, Relaxed)
                   new_area = true

           mem = unsafe { sys_mmap(null_mut(), req, PROT_READ | PROT_WRITE,
                                   MAP_PRIVATE | MAP_ANONYMOUS, -1, 0) }
           if mem == MAP_FAILED as *mut c_void || !new_area:
               bump_lock_release()
               return if mem == MAP_FAILED as *mut c_void { null_mut() } else { mem }

           BUMP_CUR.store(mem as usize, Relaxed)
           BUMP_END.store(mem as usize + req, Relaxed)

6. 从当前区域分配（bump）
   p = BUMP_CUR.load(Relaxed) as *mut c_void
   BUMP_CUR.store(p as usize + n, Relaxed)

7. 解锁并返回
   bump_lock_release()
   return p
```

#### 关键参数
- `BUMP_MMAP_STEP` 初始为 0，每次创建新 mmap 区域最多递增到 `MMAP_STEP_MAX = 12`
- `MMAP_STEP_MAX = 12` 对应 `PAGE_SIZE << 6 = 64 * PAGE_SIZE` 的最大几何增长
- 浪费比例阈值 `req - n > req / WASTE_THRESHOLD_DENOM`（即浪费 > 12.5%）触发独立 mmap 区域策略
- 新区域最小尺寸: `PAGE_SIZE << (step / 2)`（几何增长因子 sqrt(2)）

#### 不变量
- 函数通过 `bump_lock_acquire` / `bump_lock_release` 保证对 `BUMP_BRK`、`BUMP_CUR`、`BUMP_END`、`BUMP_MMAP_STEP` 的互斥访问
- `BUMP_CUR` 始终满足 `BUMP_BRK <= BUMP_CUR <= BUMP_END`
- 分配的指针始终满足最小对齐要求
- 函数不持有锁退出（包括所有失败路径）

#### 内部依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `bump_lock_acquire` / `bump_lock_release` | 本模块 mod.rs | 自旋锁获取/释放 |
| `sys_brk` | syscalls.rs | brk 系统调用 |
| `sys_mmap` | syscalls.rs | mmap 系统调用 |
| `check_stack_collision` | stack_check.rs | 栈冲突检测 |
| `PAGE_SIZE` | 本模块全局 AtomicU32 | 运行时页面大小 |
| `BUMP_BRK` / `BUMP_CUR` / `BUMP_END` | 本模块全局 | 分配器状态 |
| `BUMP_MMAP_STEP` | 本模块全局 AtomicU8 | mmap 几何增长步数 |
| `ALIGN` / `MMAP_STEP_MAX` / `WASTE_THRESHOLD_DENOM` | 本模块常量 | 算法参数 |
| `usize::MAX` | core | size_t 的最大值 |
| `ENOMEM` / `MAP_FAILED` / `PROT_*` / `MAP_*` | 本模块常量 | 系统常量 |

**Rust 设计要点**:
- C 的 `__syscall(SYS_brk, ...)` 被替换为 rusl 自定义的 `sys_brk` 内联汇编封装
- C 的 `__mmap(...)` 被替换为 rusl 自定义的 `sys_mmap` 内联汇编封装
- C 的 `LOCK(lock)` / `UNLOCK(lock)` 被替换为 `bump_lock_acquire()` / `bump_lock_release()`
- 所有模块级静态变量使用 Rust `Atomic*` 类型，确保跨线程可见性和正确的内存顺序
- 内部使用 `page_align` 辅助函数（页对齐向上取整），定义为模块级内联函数

---

## 六、内部导出函数 (mod.rs)

### __libc_malloc_impl -- 弱符号分发

```rust
/// `__simple_malloc` 的弱符号别名，是 libc 内部 malloc 实现的间接入口。
///
/// 对应 C 的 `weak_alias(__simple_malloc, __libc_malloc_impl)`。
/// 若完整 malloc 实现（mallocng）提供同名的强符号定义，则在链接时覆盖此弱符号。
#[no_mangle]
unsafe extern "C" fn __libc_malloc_impl(n: usize) -> *mut c_void;
```

[Visibility]: Internal (不导出给用户) -- musl 内部弱符号。被 libc 内部模块（如 `calloc`、`strdup` 等）通过 `__libc_malloc` 间接调用。

#### 语义
编译/链接层间接跳板。实现委托：`__libc_malloc_impl(n) = simple_malloc(n)`。

**注意**: Rust 的 `#[no_mangle]` 默认产生强符号。对于弱符号需求，rusl 编译流程需要使用链接器脚本或 `.c` 包装（在 C 端定义 `weak_alias` 引用 Rust 函数），或者在 Rust nightly 中使用 `#[linkage = "weak"]` 属性。在 spec 层面约定此符号为弱符号即可。

#### 前置/后置条件
同 `simple_malloc`。

---

### __libc_malloc -- libc 内部 malloc 入口

```rust
/// libc 内部 malloc 统一入口，间接委托给 `__libc_malloc_impl`。
///
/// 对应 C 的 `__libc_malloc`。
/// 间接调用的设计使得运行时替换 malloc 实现成为可能。
#[no_mangle]
unsafe extern "C" fn __libc_malloc(n: usize) -> *mut c_void;
```

[Visibility]: Internal (不导出给用户) -- musl 内部 API，被 libc 内部函数（如 `calloc`、`strdup`、`printf` 系列等）在完整 malloc 初始化前调用。

#### 意图
为 libc 内部使用者提供统一的 `malloc` 调用入口。当完整 malloc 初始化完毕、替换 `__libc_malloc_impl` 的强符号后，本函数无需修改即可自动路由到新实现。

#### 前置条件
- `n` 为请求分配的大小
- `__libc_malloc_impl` 符号已解析（弱符号至少由 `simple_malloc` 提供）

#### 后置条件
- 返回值与 `__libc_malloc_impl(n)` 一致
- 所有前置/后置条件继承自 `__libc_malloc_impl` 的当前绑定实现

#### 实现
```rust
unsafe extern "C" fn __libc_malloc(n: usize) -> *mut c_void {
    __libc_malloc_impl(n)
}
```

#### 依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `__libc_malloc_impl` | 本模块弱符号 | 实际的分配函数 |

---

## 七、对外导出函数 (mod.rs)

### default_malloc / malloc -- POSIX 标准 malloc

```rust
/// POSIX malloc 的内部实现，通过弱符号导出为 `malloc`。
///
/// 直接委托给 `__libc_malloc_impl`。
fn default_malloc(size: usize) -> *mut c_void {
    unsafe { __libc_malloc_impl(size) }
}
```

```rust
/// POSIX.1-2001 标准 malloc 函数。
///
/// 对应 C 的 `weak_alias(default_malloc, malloc)`。
/// 通常被完整 malloc 实现（mallocng 或 oldmalloc）的强符号覆盖。
#[no_mangle]
unsafe extern "C" fn malloc(size: usize) -> *mut c_void;
```

[Visibility]: **Public** -- POSIX.1-2001 标准函数，声明于 `<stdlib.h>`。

#### 意图
提供符合 POSIX 标准的动态内存分配接口。在正常 musl 构建中，此弱符号被完整 malloc 实现的强符号 `malloc` 覆盖；本模块版本仅作为链接时回退（fallback）或早期启动阶段的临时实现。

#### 前置条件
- `size` 为请求分配的字节数
- 若 `size == 0`，行为由实现定义（本实现返回一个有效指针，等同于 `size = 1`）

#### 后置条件
- **Case 1 -- 分配成功**:
  - 返回值: 指向至少 `size` 字节已分配内存的指针，适当对齐，内容未初始化
  - 返回的指针可安全传递给 `free()`、`realloc()` 等函数
- **Case 2 -- 分配失败**:
  - 返回值: `core::ptr::null_mut()`
  - `errno` 设置为 `ENOMEM`

#### 实现
```rust
unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    __libc_malloc_impl(size)
}
```

#### 不变量
- 函数无内部状态，不持有锁跨越调用边界
- 线程安全由 `__libc_malloc_impl` 保证

#### 依赖
| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `__libc_malloc_impl` | 本模块弱符号 | 实际的分配逻辑 |

---

## 跨模块依赖汇总

### rusl 外部依赖（需由 rusl 其他模块提供）

| 依赖符号 | 相应 rusl 模块 | 用途 |
|----------|---------------|------|
| `PAGE_SIZE` (初始化) | `crt/init` 或 `crt/auxv` | 运行时页面大小，由启动代码在进入 main 前设置 |
| `AUXV` (初始化) | `crt/init` 或 `crt/auxv` | 内核辅助向量指针，用于栈区间检测 |
| `__bump_lockptr` (消费方) | `process/fork` | fork 安全锁指针，在 fork() 前加锁 |
| `errno` 设置机制 | `errno/__errno_location` | 错误码设置（通过 `*__errno_location() = ENOMEM`） |

### rusl 内部依赖（本模块自行提供）

| 依赖 | 来源 | 说明 |
|------|------|------|
| `bump_lock_acquire` / `bump_lock_release` | mod.rs (内联) | 自旋锁实现，使用 `AtomicI32` |
| `sys_brk` | syscalls.rs | brk 系统调用，使用 `asm!` |
| `sys_mmap` | syscalls.rs | mmap 系统调用，使用 `asm!` |
| `check_stack_collision` | stack_check.rs | 栈冲突检测 |
| `simple_malloc` | bump.rs | bump 分配器核心逻辑 |
| 所有 `Atomic*` 静态变量 | mod.rs | 分配器全局状态 |

### 被移除的 C 依赖

以下 C 实现依赖在 Rust 设计中已被替换或消除：

| C 依赖 | Rust 替换方案 | 说明 |
|---------|--------------|------|
| `__lock` / `__unlock` (src/thread/__lock.c) | `AtomicI32` + `spin_loop()` | Rust 安全自旋锁替代 musl 的线程锁 |
| `__syscall` (src/internal/syscall.h) | `asm!("syscall")` 内联汇编 | rusl 直接使用内联汇编发起 syscall |
| `__mmap` (src/mman/mmap.c) | `sys_mmap` 内联汇编封装 | rusl 模块内定义的专用 mmap 封装 |
| `LOCK(lock)` / `UNLOCK(lock)` 宏 (lock.h) | `bump_lock_acquire()` / `bump_lock_release()` | Rust 内联函数替代 C 宏 |
| `libc.page_size` (struct __libc 字段) | `PAGE_SIZE: AtomicU32` | Rust 全局变量，语义等价 |
| `libc.auxv` (struct __libc 字段) | `AUXV: AtomicPtr<c_ulong>` | Rust 全局变量，语义等价 |
| `traverses_stack_p` (C static 函数) | `check_stack_collision` | 安全 Rust 重新设计 |
| `weak_alias` 宏 (libc.h) | `#[no_mangle]` + 链接器弱符号支持 | Rust/C 混合构建的链接策略 |
| `volatile int lock[1]` | `AtomicI32` | Rust 原子类型替代 C volatile |
| `SIZE_MAX` | `usize::MAX` | Rust 标准库常量 |
| `<errno.h>` (C 标准头) | 模块内 `ENOMEM` 常量 | rusl no_std 环境自行定义 |
| `<sys/mman.h>` (C 标准头) | 模块内常量 | rusl no_std 环境自行定义所有 mmap 常量 |

---

## 设计备注

1. **弱符号覆盖机制**: musl 采用静态链接期弱/强符号替换策略。rusl 使用 Rust 的 `#[no_mangle]` 属性导出符号，弱符号特性通过链接器层面实现（如 GNU ld 的 `--defsym` 或链接器脚本的 `PROVIDE` 指令）。在实际构建中，若链接了完整 malloc 实现提供的强符号 `malloc` 和 `__libc_malloc_impl`，本模块的对应定义将被覆盖，`simple_malloc` 代码可在 LTO 死代码消除阶段被完全移除。

2. **bump 分配器的语义限制**: `simple_malloc` 是"纯增量"分配器，分配的内存不可被释放。它仅设计用于：
   - libc 早期初始化阶段（在完整 malloc 接管之前分配少量持久对象）
   - 极端链接配置下作为最后的 fallback

3. **brk 优先策略**: 优先通过 `sys_brk` 扩展堆，因为 `brk` 在数据段范围内连续，缓存局部性优于 `sys_mmap` 的随机地址分配。仅当 `brk` 可能穿过栈区域或 `brk` 系统调用失败时回退到 `sys_mmap`。

4. **mmap 区域几何增长**: `BUMP_MMAP_STEP` 的几何增长策略（`min = PAGE_SIZE << (step/2)`）使连续的大块请求倾向于复用同一 mmap 区域，减少系统调用次数和 VMA 碎片。

5. **no_std 兼容性**: 整个模块仅依赖 `core`（`core::sync::atomic::*`、`core::ptr`、`core::arch::asm`、`core::hint`、`core::ffi::*`），不依赖 `std`。所有 POSIX 常量（`ENOMEM`、`PROT_*`、`MAP_*`）在模块内自行定义为常量，不依赖 C 头文件或 `libc` crate。

6. **errno 设置**: rusl 中 `errno` 通过 `*__errno_location() = ENOMEM` 方式设置。`__errno_location` 是 rusl 自身的内部导出函数，返回线程局部 `errno` 变量的地址。`lite_malloc` 模块需在设置 errno 时调用 `rusl_errno::set_errno(ENOMEM)` 或等价的内部接口。

---

## 依赖规约

/* Rely */
[RELY]
Predefined Structures/Functions:
  // === rusl 内部依赖（其他 rusl 模块提供） ===
  fn __errno_location() -> *mut c_int;       // 依赖1: errno 线程局部变量地址，see errno/__errno_location
  fn set_errno(e: c_int);                    // 依赖2: errno 设置的便捷封装（委托给 __errno_location）

  // === 全局状态初始化依赖 ===
  // PAGE_SIZE (AtomicU32)                    // 依赖3: 由 crt/init 在进入 main 前初始化
  // AUXV (AtomicPtr<c_ulong>)                // 依赖4: 由 crt/init 在进入 main 前初始化

  // === 消费方依赖 ===
  // __bump_lockptr                           // 依赖5: 被 process/fork 模块引用，用于 fork 前加锁

  // === core crate 依赖 ===
  // core::sync::atomic::{AtomicI32, AtomicU32, AtomicU8, AtomicUsize, AtomicPtr, Ordering}
                                              // 依赖6: 无锁原子类型和内存顺序
  // core::arch::asm!                         // 依赖7: 内联汇编宏，用于发起 syscall 指令
  // core::hint::spin_loop                    // 依赖8: 自旋锁忙等待提示
  // core::ptr::{null_mut, ...}              // 依赖9: 空指针常量

  // === 链接器依赖 ===
  // 弱符号机制 (PROVIDE / --defsym)          // 依赖10: 构建系统/链接器脚本提供弱符号支持

[GUARANTEE]
Exported Interface (Public):
  /// POSIX.1-2001 标准 malloc 函数（弱符号）
  #[no_mangle]
  unsafe extern "C" fn malloc(size: usize) -> *mut c_void;

Exported Interface (Internal, ABI-stable):
  /// libc 内部 malloc 实现入口（弱符号，可被强符号覆盖）
  #[no_mangle]
  unsafe extern "C" fn __libc_malloc_impl(n: usize) -> *mut c_void;

  /// libc 内部 malloc 间接调用入口（强符号，不可被覆盖）
  #[no_mangle]
  unsafe extern "C" fn __libc_malloc(n: usize) -> *mut c_void;

  /// fork 安全锁指针（被 process/fork 引用）
  /// 实际导出类型为 *mut c_int，指向 BUMP_LOCK
  #[no_mangle]
  static mut __bump_lockptr: *mut c_int;

Internal Interface (pub(crate), 非 ABI 稳定):
  // === mod.rs ===
  // static BUMP_LOCK: AtomicI32            -- 分配器自旋锁
  // static BUMP_BRK: AtomicUsize            -- brk 值
  // static BUMP_CUR: AtomicUsize            -- 当前分配游标
  // static BUMP_END: AtomicUsize            -- 当前区域末尾
  // static BUMP_MMAP_STEP: AtomicU8        -- mmap 几何增长步数
  // static PAGE_SIZE: AtomicU32            -- 运行时页面大小
  // static AUXV: AtomicPtr<c_ulong>        -- 内核辅助向量指针
  // fn bump_lock_acquire()                  -- 获取自旋锁
  // fn bump_lock_release()                  -- 释放自旋锁
  // fn page_align(x: usize) -> usize        -- 页对齐向上取整

  // === syscalls.rs ===
  // unsafe fn sys_brk(addr: usize) -> usize               -- brk 系统调用
  // unsafe fn sys_mmap(addr: *mut c_void, len: usize, prot: c_int,
  //                     flags: c_int, fd: c_int, offset: isize) -> *mut c_void  -- mmap 系统调用

  // === stack_check.rs ===
  // fn check_stack_collision(old: usize, new: usize) -> bool  -- 栈冲突检测

  // === bump.rs ===
  // fn simple_malloc(n: usize) -> *mut c_void             -- bump 分配器核心

  // === 内部常量 ===
  // const ALIGN: usize = 16
  // const STACK_ESTIMATE: usize = 8 << 20
  // const MMAP_STEP_MAX: u8 = 12
  // const WASTE_THRESHOLD_DENOM: usize = 8
  // const SYS_BRK: isize = ... (按架构)
  // const SYS_MMAP: isize = ... (按架构)
  // const ENOMEM: i32 = 12
  // const MAP_FAILED: usize = !0
  // const PROT_READ: i32 = 0x1
  // const PROT_WRITE: i32 = 0x2
  // const MAP_PRIVATE: i32 = 0x02
  // const MAP_ANONYMOUS: i32 = 0x20
  // const OVER_MARGIN: usize = 4096