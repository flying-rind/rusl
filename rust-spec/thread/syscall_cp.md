# syscall_cp — Rust 接口归约

> 此文件为空文件（0 字节）。`__syscall_cp_c` 的实际实现在架构相关的汇编源文件中（如 `x86_64/__syscall_cp.s`）。此 C 源文件在 musl 构建中作为各架构汇编实现的占位标记。

---

## 原始 C 接口

此文件不包含任何符号定义。在 musl 的构建系统中，`syscall_cp.c` 作为各架构 `__syscall_cp_c` 汇编实现的占位标记存在。单线程构建中，`__syscall_cp.c` 中的弱别名确保 `__syscall_cp_c` 解析为简单的 `sccp`（直接 `__syscall`）。

---

## Rust 实现说明

**此文件不需要 Rust 实现。** `__syscall_cp_c` 的实际实现位于架构相关的汇编文件中。rusl 中对应的符号由 `__syscall_cp.md` 模块提供。

在 rusl 的构建中：
- 单线程构建：`__syscall_cp_c` 回退为 `sccp_c`（直接调用 `__syscall`），不包含取消点检查
- 多线程构建：`__syscall_cp_c` 由架构相关的汇编源文件提供，在系统调用前检查取消点

---

## 依赖关系

```
syscall_cp.c (空文件 — 被架构特定汇编替代)
  替代者: arch/<arch>/__syscall_cp.s
  回退:   __syscall_cp.c → sccp_c (作为 __syscall_cp_c 的默认实现)
```

---

## 可见性

[Visibility]: Internal (不导出) — musl/rusl 构建系统内部文件，不包含任何用户可见符号

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  无直接依赖 — 此文件不包含任何功能代码

[GUARANTEE]
Exported Interface:
  无 — 此文件不导出任何符号，所有功能由 __syscall_cp 模块和架构相关汇编文件提供
