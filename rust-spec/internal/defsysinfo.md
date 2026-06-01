# defsysinfo 模块规约 (Rust)

> **源 C spec**: `/home/mangp/桌面/OS/musl/src/internal/spec/defsysinfo.md`
> **复杂度等级**: Level 1（简单模块 — 单个全局变量定义）

---

## 依赖图

```
__SYSINFO (全局静态变量) → (无内部依赖)
```

该模块极为简单：仅定义一个全局静态变量 `__SYSINFO`，不调用任何函数，不依赖任何内部结构体。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::sync::atomic::AtomicUsize` | Rust core 库 | 提供线程安全的原子访问语义 |

### 为什么使用 `AtomicUsize`

C 版本的 `__sysinfo` 类型为 `size_t`，使用 `volatile` 限定并由 `__init_libc` / `__init_tls` 写入，由 `__syscall` 族汇编函数读取。在 Rust 中，使用 `AtomicUsize` 替代裸 `usize` 的原因：

1. **内存顺序保证**: `AtomicUsize` 提供明确的 acquire/release 语义，确保初始化线程的写入对所有后续读取者可见。
2. **避免数据竞争**: `static mut` 的访问在 Rust 中需要 `unsafe`；`AtomicUsize` 提供了安全的 `load`/`store` 操作。
3. **编译器优化防护**: `AtomicUsize` 阻止编译器将读取操作优化掉或重排跨线程访问。

---

## 全局变量规约

---

### `__SYSINFO`

```rust
// Rust 声明 — rusl 内部全局静态变量，不对外导出
pub(crate) static __SYSINFO: AtomicUsize = AtomicUsize::new(0);
```

[Visibility]: Internal — rusl 运行时内部全局状态，由 `__init_libc` / `__init_tls` 初始化，被 `__syscall` 族函数读取。Linux 用户空间程序不应直接访问。

#### 功能意图 (Intent)

存储 ELF 辅助向量（auxiliary vector）中的 `AT_SYSINFO` 或 `AT_SYSINFO_EHDR` 条目值。该值指向 Linux vDSO（virtual Dynamic Shared Object）中的内核辅助代码页地址，用于加速某些系统调用（如 `gettimeofday`、`clock_gettime` 等），避免陷入内核态。

#### 初始值

`0`（等价于 C 版本的 BSS 段零初始化）。

#### 后置条件（初始化后）

- 若内核提供 vDSO 且含有 `AT_SYSINFO_EHDR` 或 `AT_SYSINFO`，则 `__SYSINFO.load(Ordering::Relaxed) != 0`
- 否则 `__SYSINFO.load(Ordering::Relaxed) == 0`，`__syscall` 族函数回退到标准 `int $0x80`/`syscall` 指令

#### 读写约定

| 操作 | 代码示例 | 内存顺序 | 说明 |
|------|----------|----------|------|
| **写入**（初始化阶段） | `__SYSINFO.store(addr, Ordering::Release)` | Release | 确保初始化完成后再被其他上下文读取 |
| **读取**（系统调用路径） | `__SYSINFO.load(Ordering::Acquire)` | Acquire | 配对 Release 写入，保证看到最新值 |
| **读取**（非关键路径） | `__SYSINFO.load(Ordering::Relaxed)` | Relaxed | 仅用于检查是否为 0，无需同步语义 |

#### 使用方

`__syscall()` / `__syscall_cp()` 汇编或内联汇编实现 —— 在进入内核前检查 `__SYSINFO` 是否为非零以决策是否使用 vDSO 快速路径。

#### 不变量 (Invariants)

- **INV-1**: `__SYSINFO` 在进程启动后仅写入一次（由 `__init_libc`），后续为只读。
- **INV-2**: `__SYSINFO == 0` 表示"未初始化"或"系统不支持 vDSO"；两种情况的行为等价（回退到标准系统调用路径）。
- **INV-3**: `__SYSINFO != 0` 时，其值为有效的用户空间可执行内存地址。

#### 与 C 实现的差异

| 方面 | C (musl) | Rust (rusl) |
|------|----------|-------------|
| 类型 | `volatile size_t` | `AtomicUsize` |
| 零初始化 | BSS 段隐式零初始化 | `AtomicUsize::new(0)` |
| 读取操作 | 普通内存读取（`volatile` 阻止优化） | `AtomicUsize::load(Ordering)` 提供内存顺序 |
| 写入操作 | 普通赋值（`volatile` 阻止优化） | `AtomicUsize::store(value, Ordering)` |
| 线程安全 | 依赖 `volatile` 语义（弱保证） | 明确的内存顺序模型 |

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `__SYSINFO` | 本模块 | 全局原子变量定义 |
| `__SYSINFO` 的写入者 | `src/env/init_libc.rs` | `__init_libc` 解析辅助向量并写入 vDSO 地址 |
| `__SYSINFO` 的读取者 | `src/syscall/mod.rs` 或等效模块 | `__syscall` 族函数在调用前读取以决策快速路径 |