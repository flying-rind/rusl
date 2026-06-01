# __shcall.c 规约

> 源文件: `/home/mangp/桌面/OS/musl/src/internal/sh/__shcall.c`
> 所属模块: musl 内部 stdio 扫描辅助（SuperH 架构变体）
> 复杂度层级: **Level 1** — 纯转发包装器，仅需前置/后置条件

---

## 依赖图

```
__shcall ──> func(arg) (函数指针间接调用)
```

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `hidden` 宏 | `features.h` | 编译器属性，跳过 |
| `void *arg` | 调用者提供 | 直接转发 |
| `int (*func)(void *)` | 调用者提供 | 直接转发 |

---

## __shcall (内部函数)

### 签名

```c
int __shcall(void *arg, int (*func)(void *));
```

### 可见性

**[Visibility]: Internal (不导出)** — `hidden` 属性声明于函数定义。这是 musl 内部用于 SuperH (sh) 架构的转发包装器，将函数指针调用从共享库 PLT 延迟绑定路径中解耦。POSIX/C 标准未定义此符号。

### 意图 (Intent)

在 SuperH 架构上，musl 的某些内部扫描函数通过函数指针间接调用 stdio 辅助函数。直接调用函数指针可能触发 PLT（Procedure Linkage Table）延迟绑定，在特定场景下（如信号处理中）导致问题。`__shcall` 作为一层转发，允许编译器将调用点内联或优化，避免 PLT 跳转。

更一般地，此模式是 musl 内部约定：通过一个包装函数间接调用所有函数指针参数，以保持控制流可预测性并避免 PLT 相关的重入问题。

### 前置条件

- `func` 不为 NULL（有效的函数指针）
- `arg` 的类型与 `func` 期望的参数类型兼容
- 若调用发生在信号处理上下文，`func` 必须是 async-signal-safe

### 后置条件

- 返回值等于 `func(arg)` 的返回值
- 函数无任何副作用（除 `func(arg)` 的副作用外）
- 此函数自身不修改 `errno`

### 不变量

`__shcall` 在任何架构上都是纯转发（恒等变换）：`__shcall(a, f) ≡ f(a)`。此不变量保证替换 `__shcall` 的语义与直接调用 `func(arg)` 完全等价。

### 系统算法 (System Algorithm)

**设计模式：间接调用解耦**

```
调用者 ──functx_ptr──> __shcall ──直接调用──> func(arg)
```

此模式在 musl 中多处使用（如 `__shcall`、`__fclose_ca` 的回调路径），目的为：

1. **PLT 隔离**: 在共享库构建中，通过 `hidden` 可见性确保 `__shcall` 的调用不经过 PLT。即使 `func` 的地址通过 PLT 获取，调用点（`__shcall` 内）是 `hidden` 的，编译器可用直接调用指令。
2. **尾调用优化**: 此函数形式为 `return func(arg)`，编译器通常将其优化为尾调用（tail call），消除额外栈帧。
3. **类型擦除接口**: `void *` 参数允许传入任意上下文，与 `int (*)(void *)` 签名的 stdio 回调兼容。

### Rust 实现注意事项

在 Rust `#![no_std]` 实现中：
- 此函数为简单的函数指针调用转发：`(func)(arg)`
- 可能被实现为零开销抽象——直接内联为 `func(arg)` 调用
- 若 rustc 优化足够，此包装器本身可以完全在编译时消除