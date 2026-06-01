# defsysinfo.c 规约

> musl 内部全局变量定义，无函数实现。

---

## 依赖图

```
__sysinfo (全局变量) → (无内部依赖)
```

该文件极为简单：仅定义全局变量 `__sysinfo`，不调用任何函数，不依赖任何内部结构体。

---

## 依赖说明

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `size_t` | `<stddef.h>`（通过 libc.h） | C 语言内建类型，跳过 |
| `hidden` 可见性属性 | musl 编译约定 | 编译器指示，非逻辑依赖 |
| `PAGE_SIZE` 宏 | `libc.h`（由 `__sysinfo` 间接提供 AT_SYSINFO） | 仅文档记录 |

---

## 全局变量规约

### __sysinfo

```c
// C 声明 (libc.h)
extern hidden size_t __sysinfo;

// C 定义 (defsysinfo.c)
size_t __sysinfo;
```

```rust
// Rust 对应声明 (rusl)
// 注：Rust 全局可变静态变量，位于内部模块，不对外导出
static mut __SYSINFO: usize;
```

**[Visibility]: Internal (不导出)** — musl 运行时内部全局状态，由 `__init_libc` / `__init_tls` 初始化，被 `__syscall` 族函数读取。Linux 用户空间程序不应直接访问。

**意图（Intent）**:
存储 ELF 辅助向量（auxiliary vector）中的 `AT_SYSINFO` 或 `AT_SYSINFO_EHDR` 条目值。该值指向 Linux vDSO（virtual Dynamic Shared Object）中的内核辅助代码页地址，用于加速某些系统调用（如 `gettimeofday`、`clock_gettime` 等），避免陷入内核态。

**初始值**: `0`（BSS 段零初始化）。

**后置条件（初始化后）**:
- 若内核提供 vDSO 且含有 `AT_SYSINFO_EHDR` 或 `AT_SYSINFO`，则 `__sysinfo != 0`
- 否则 `__sysinfo == 0`，`__syscall` 族函数回退到标准 `int $0x80`/`syscall` 指令

**使用方**: `__syscall()` / `__syscall_cp()` 汇编实现 —— 在进入内核前检查 `__sysinfo` 是否为非零以决策是否使用 vDSO 快速路径。