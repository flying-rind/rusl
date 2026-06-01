# atomic 模块规约 (Rust)

> **源 C spec**: `/home/mangp/桌面/OS/musl/src/internal/spec/atomic.md`
> **复杂度等级**: Level 2（中等复杂度 — 类型封装 + 少量辅助函数）

---

## 依赖图

```
core::sync::atomic ──> atomic 模块 ──> 使用者（rusl 其他模块）
                          │
                          ├── AtomicI32 / AtomicU32 / AtomicUsize / AtomicPtr 等
                          ├── Ordering 重导出 (SeqCst, Relaxed 等)
                          ├── a_ctz_32 / a_ctz_64 / a_clz_32 / a_clz_64 (位操作)
                          ├── a_crash (崩溃函数)
                          └── a_and_64 / a_or_64 (拆分为 32 位操作的 64 位原子位操作)
```

在 Rust 实现中，musl C 的 `atomic.h` 宏和 `static inline` 函数全部由 `core::sync::atomic` 标准原语替代。Rust 的原子类型（`AtomicI32`、`AtomicU32`、`AtomicUsize` 等）已提供编译器内建的原子指令生成，无需手写 LL/SC 或 CAS 循环。架构差异由 Rust 编译器和 LLVM 后端处理，无需像 C 版本那样依赖 `atomic_arch.h`。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::sync::atomic` | Rust core 库（`#![no_std]` 兼容） | 原子操作基础原语 |
| `core::intrinsics` | Rust core 库 | `abort()` 用于 `a_crash` |

---

## 设计原则

Rust 版本采取**类型封装**策略而非 C 版本的**宏 + `static inline`**策略：

1. 所有 32 位原子操作直接使用 `AtomicI32`（或 `AtomicU32`）的标准方法。
2. `a_cas`、`a_swap`、`a_fetch_add` 等宏不再单独存在，调用者直接使用 `AtomicI32::compare_exchange()`、`AtomicI32::swap()`、`AtomicI32::fetch_add()` 等方法。
3. 仅保留 C 实现中无法直接映射的辅助函数（位操作 `a_ctz_*`/`a_clz_*`、崩溃函数 `a_crash`、64 位拆分操作 `a_and_64`/`a_or_64`）。

---

## 符号规约

---

### 类型别名和重导出

```rust
// Rust — 原子类型重导出（语义别名，保持与 musl 的术语一致）
pub(crate) use core::sync::atomic::AtomicI32 as a_int;
pub(crate) use core::sync::atomic::AtomicU32 as a_uint;
pub(crate) use core::sync::atomic::AtomicUsize as a_size;
pub(crate) use core::sync::atomic::AtomicPtr as a_ptr;

// 默认内存顺序 — musl 原子操作始终使用 SeqCst
pub(crate) const A_ORDER: Ordering = Ordering::SeqCst;
```

[Visibility]: Internal — rusl 原子基础设施，用于内部代码风格统一。

#### 功能意图 (Intent)

提供与 musl C 代码中 `a_inc(&x)`、`a_dec(&x)`、`a_cas(&x, t, s)` 等模式语义等价且简洁的别名，降低从 C 代码到 Rust 代码的迁移心智负担。`a_int` 类型的 `.fetch_add(1, A_ORDER)` 等价于 C 的 `a_inc(&x)`。

---

### C 到 Rust 映射速查表

| C musl 原子操作 | Rust 等价写法 | 内存顺序 |
|-----------------|---------------|----------|
| `a_cas(p, t, s)` | `(*p).compare_exchange(t, s, A_ORDER, A_ORDER)` | SeqCst |
| `a_swap(p, v)` | `(*p).swap(v, A_ORDER)` | SeqCst |
| `a_fetch_add(p, v)` | `(*p).fetch_add(v, A_ORDER)` | SeqCst |
| `a_fetch_and(p, v)` | `(*p).fetch_and(v, A_ORDER)` | SeqCst |
| `a_fetch_or(p, v)` | `(*p).fetch_or(v, A_ORDER)` | SeqCst |
| `a_and(p, v)` | `{ (*p).fetch_and(v, A_ORDER); }` | SeqCst |
| `a_or(p, v)` | `{ (*p).fetch_or(v, A_ORDER); }` | SeqCst |
| `a_inc(p)` | `{ (*p).fetch_add(1, A_ORDER); }` | SeqCst |
| `a_dec(p)` | `{ (*p).fetch_sub(1, A_ORDER); }` | SeqCst |
| `a_store(p, v)` | `(*p).store(v, A_ORDER)` | SeqCst |
| `a_barrier()` | `core::sync::atomic::fence(Ordering::SeqCst)` | SeqCst |
| `a_spin()` | `core::sync::atomic::spin_loop_hint()` | Relaxed |
| `a_cas_p(p, t, s)` | `(*p).compare_exchange(t, s, A_ORDER, A_ORDER)` (AtomicPtr) | SeqCst |

注意：Rust 要求原子操作在 `&Atomic*` 上调用，而非 C 风格裸指针。调用者需要持有对 `AtomicI32` 等的引用。

---

### `a_and_64`

```rust
// Rust 声明 — 内部辅助函数
pub(crate) fn a_and_64(p: &AtomicU64, v: u64);
```

[Visibility]: Internal — rusl 原子操作辅助函数。

#### 系统算法 (System Algorithm)

与 C 版本相同：将 64 位按位与分解为两个 32 位原子操作。

```rust
fn a_and_64(p: &AtomicU64, v: u64) {
    let lo = v as u32;
    let hi = (v >> 32) as u32;
    // 将 AtomicU64 的地址视为两个 AtomicU32（小端序布局）
    let p_lo = unsafe { &*(p as *const AtomicU64 as *const AtomicU32) };
    let p_hi = unsafe { &*(p as *const AtomicU64 as *const AtomicU32).add(1) };
    if lo != 0xFFFF_FFFF { p_lo.fetch_and(lo, A_ORDER); }
    if hi != 0xFFFF_FFFF { p_hi.fetch_and(hi, A_ORDER); }
}
```

**Key Insight**: 此算法**不是原子 64 位操作**——两个 32 位操作之间存在竞态窗口。此函数仅用于标志位清除场景，在 musl 的使用上下文中，竞态条件是可接受的。

#### 前置条件 (Preconditions)

- **PRE-1**: `p` 指向有效的 4 字节对齐（或更优）的 `AtomicU64`。

#### 后置条件 (Postconditions)

- **POST-1**: `*p` 的低 32 位被原子地按位与 `(u32)v`（当 `v` 的低 32 位不是全 1 时）。
- **POST-2**: `*p` 的高 32 位被原子地按位与 `(u32)(v >> 32)`（当 `v` 的高 32 位不是全 1 时）。
- **POST-3**: 低 32 位和高 32 位的修改之间**没有原子性保证**。

---

### `a_or_64`

```rust
// Rust 声明 — 内部辅助函数
pub(crate) fn a_or_64(p: &AtomicU64, v: u64);
```

[Visibility]: Internal — rusl 原子操作辅助函数。

#### 系统算法 (System Algorithm)

将 64 位按位或分解为两个 32 位原子操作。若某 32 位半为 0 则跳过该半（`x | 0 == x`）。

#### 前置条件 (Preconditions)

- **PRE-1**: `p` 指向有效的 4 字节对齐的 `AtomicU64`。

#### 后置条件 (Postconditions)

- **POST-1**: `*p` 的低 32 位被原子地按位或 `(u32)v`（当 `v` 的低 32 位非 0 时）。
- **POST-2**: `*p` 的高 32 位被原子地按位或 `(u32)(v >> 32)`（当 `v` 的高 32 位非 0 时）。
- **POST-3**: 低 32 位和高 32 位之间没有 64 位原子性。

---

### `a_or_l`

```rust
// Rust 声明 — 内部辅助函数（泛型化）
pub(crate) fn a_or_l(p: &AtomicUsize, v: usize);
```

[Visibility]: Internal — rusl 原子操作辅助函数。

#### 功能意图 (Intent)

与平台字长匹配的原子按位或。直接使用 `AtomicUsize::fetch_or(v, A_ORDER)`，无需条件分发——Rust 的 `AtomicUsize` 自动匹配平台字长。

---

### `a_ctz_32`

```rust
// Rust 声明 — 内部辅助函数
pub(crate) fn a_ctz_32(x: u32) -> u32;
```

[Visibility]: Internal — rusl 位操作辅助函数。

#### 系统算法 (System Algorithm)

直接调用 Rust 内建方法 `x.trailing_zeros()`，由 LLVM 编译为目标架构最优指令（x86_64 的 `TZCNT`/`BSF`、aarch64 的 `RBIT`+`CLZ` 等）。无需 De Bruijn 序列查表。

#### 前置条件 (Preconditions)

- **PRE-1**: `x` 不为 0。调用者负责确保 `x != 0`（`a_ctz_32(0)` 的结果为 32，但语义上不保证）。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `x` 的二进制表示中最低位的 1 所在的位置（0 = bit 0, 31 = bit 31）。

---

### `a_ctz_64`

```rust
// Rust 声明 — 内部辅助函数
pub(crate) fn a_ctz_64(x: u64) -> u32;
```

[Visibility]: Internal — rusl 位操作辅助函数。

#### 系统算法 (System Algorithm)

直接调用 `x.trailing_zeros()`。LLVM 在 32 位平台上自动分解为两个 32 位操作，在 64 位平台上使用原生 64 位指令。

#### 前置条件 (Preconditions)

- **PRE-1**: `x` 不为 0。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `x` 的二进制表示中最低位的 1 所在的位置（0..63）。

---

### `a_ctz_l`

```rust
// Rust 声明 — 内部辅助函数
pub(crate) fn a_ctz_l(x: usize) -> u32;
```

[Visibility]: Internal — rusl 位操作辅助函数。

#### 功能意图 (Intent)

与平台字长匹配的尾随零计数。直接调用 `x.trailing_zeros()`，自动适配 `usize` 的字长。

---

### `a_clz_32`

```rust
// Rust 声明 — 内部辅助函数
pub(crate) fn a_clz_32(x: u32) -> u32;
```

[Visibility]: Internal — rusl 位操作辅助函数。

#### 系统算法 (System Algorithm)

直接调用 `x.leading_zeros()`。编译为 `LZCNT`/`BSR`（x86_64）或 `CLZ`（aarch64）。

#### 前置条件 (Preconditions)

- **PRE-1**: `x` 可以为 0。当 `x == 0` 时，返回 32。

#### 后置条件 (Postconditions)

- **POST-1**: 当 `x != 0` 时，返回 `x` 最高位 1 之前的前导零数（0..31）。
- **POST-2**: 当 `x == 0` 时，返回 32。

---

### `a_clz_64`

```rust
// Rust 声明 — 内部辅助函数
pub(crate) fn a_clz_64(x: u64) -> u32;
```

[Visibility]: Internal — rusl 位操作辅助函数。

#### 系统算法 (System Algorithm)

直接调用 `x.leading_zeros()`。LLVM 自动处理 32 位兼容路径。

#### 前置条件 (Preconditions)

- **PRE-1**: `x` 可以为 0。当 `x == 0` 时，返回 64。

#### 后置条件 (Postconditions)

- **POST-1**: 当 `x != 0` 时，返回 `x` 的最高位 1 之前的前导零数（0..63）。
- **POST-2**: 当 `x == 0` 时，返回 64。

---

### `a_crash`

```rust
// Rust 声明 — 内部辅助函数
pub(crate) fn a_crash() -> !;
```

[Visibility]: Internal — rusl 崩溃函数。

#### 功能意图 (Intent)

触发立即程序终止。使用 `core::intrinsics::abort()` 替代 C 版本的写入空指针，语义更明确且不触发 UB。

#### 后置条件 (Postconditions)

- **POST-1**: 程序终止（通过 `abort` 信号）。
- **POST-2**: 此函数永不返回（Rust 类型 `!` 表达该语义）。

---

## 全局不变量 (Global Invariants)

- **GINV-1 (内存顺序一致性)**: rusl 中所有原子操作默认使用 `Ordering::SeqCst`，与 musl C 原语的顺序一致性语义保持一致。
- **GINV-2 (无锁保证)**: 所有 `core::sync::atomic` 原语均为无锁实现，依赖于硬件原子指令。
- **GINV-3 (架构无关)**: 与 C 版本不同，Rust 实现不需要 `atomic_arch.h`——架构差异由 `core::sync::atomic` 和 LLVM 后端透明处理。
- **GINV-4 (零成本抽象)**: 所有 `core::sync::atomic` 方法调用均编译为单条原子指令（或同等的 LL/SC 序列），无额外函数调用开销。

---

## Rust 与 C 实现的关键差异

| 方面 | C (musl atomic.h) | Rust (core::sync::atomic) |
|------|-------------------|---------------------------|
| 原子变量类型 | `volatile int`（普通 int + volatile 限定） | `AtomicI32`（不透明原子类型） |
| 操作方式 | 宏/static inline 函数（如 `a_inc(&x)`） | 方法调用（如 `x.fetch_add(1, Ordering::SeqCst)`） |
| 架构适配 | 依赖 `atomic_arch.h` 条件编译 | LLVM 后端自动选择指令 |
| 内存顺序 | 隐式（全 SeqCst） | 显式 `Ordering` 参数 |
| 编译器屏障 | 需要 `volatile` + `a_barrier` | 编译器自动识别原子语义 |
| 位操作 | De Bruijn 查表 / 软件二分搜索 | 内建 `leading_zeros()` / `trailing_zeros()` |
| 指针原子 | `a_cas_p` 指针到 int 强制转换（仅 32 位安全） | `AtomicPtr<T>::compare_exchange()`（任意指针大小） |

---

## 跨模块依赖

| 符号 | 来源 | 关系 |
|------|------|------|
| `core::sync::atomic::*` | Rust core lib | 所有原子操作的底层实现 |
| `core::sync::atomic::spin_loop_hint` | Rust core lib | `a_spin` 的等效实现 |
| `core::intrinsics::abort` | Rust core lib | `a_crash` 的等效实现 |