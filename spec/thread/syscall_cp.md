# syscall_cp.c 规约

> 此文件为空文件（0 字节）。`__syscall_cp_c` 的实际实现在架构相关的汇编源文件中（如 `x86_64/__syscall_cp.s`）。`__syscall_cp.c` 中的 `weak_alias(sccp, __syscall_cp_c)` 提供默认回退。

---

## 说明

在 musl 的构建系统中，`syscall_cp.c` 作为各架构 `__syscall_cp_c` 汇编实现的占位标记存在。在多线程构建中，此文件被对应的汇编 `.s` 文件替代。

单线程构建中，`__syscall_cp.c` 中的弱别名确保 `__syscall_cp_c` 解析为简单的 `sccp`（直接 `__syscall`），不包含取消点检查。

---

## 依赖关系

```
syscall_cp.c (空文件 — 被架构特定汇编替代)
  替代者: arch/<arch>/__syscall_cp.s
  回退:   __syscall_cp.c → weak_alias(sccp, __syscall_cp_c)
```

---

## 可见性

[Visibility]: Internal (不导出) — musl 构建系统内部文件，不包含任何用户可见符号
