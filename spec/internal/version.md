# version.c 规约

> 源文件: `/home/mangp/桌面/OS/musl/src/internal/version.c`
> 所属模块: musl 内部版本信息
> 复杂度层级: **Level 1** — 纯常量定义，仅需前置/后置条件

---

## 依赖图

```
__libc_version ──> VERSION 宏 (来自 version.h, 构建时生成)
```

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `VERSION` 宏 | `obj/src/internal/version.h` (构建时生成) | 外部构建产物，跳过 |
| `libc.h` 声明 | `src/internal/libc.h` | 同一内部头文件（声明）, 跳过 |

---

## 构建时依赖：version.h

`version.h` 不在源目录中，而是在构建时由 musl 的 `Makefile` 生成于 `obj/src/internal/version.h`。其内容类似：

```c
#define VERSION "1.2.6-git-11-g5122f9f3"
```

此字符串由 `git describe` 或 `Makefile` 中的工具脚本生成，反映当前的 musl 版本标识。

---

## __libc_version (内部全局常量)

### 签名

```c
const char __libc_version[];
```

### 可见性

**[Visibility]: Internal (不导出)** — `extern hidden const char __libc_version[]` 声明于 `libc.h`。该字符串供 musl 内部使用（如 `__libc_start_main` 可通过 auxv 传递版本信息），也可被调试工具读取。它不是 POSIX/C 标准定义的符号，不应当被用户程序直接引用。

### 声明位置

```c
// libc.h:
extern hidden const char __libc_version[];
```

### 意图 (Intent)

提供 libc 自身的版本字符串，用于：
1. **运行时标识**: 在程序启动时，musl 可通过 `__init_libc` 将版本信息放入辅助向量或调试结构中
2. **兼容性检查**: 理论上可被用于运行时检测 libc 版本（尽管 musl 不鼓励此类做法）
3. **调试/日志**: crash reporter 或调试器可读取此符号获取 libc 版本信息

### 前置条件

无。这是一个编译时常量/只读全局变量，程序启动前即已确定其内容。

### 后置条件

- `__libc_version` 指向一个 NUL 结尾的字符串，内容等于 `VERSION` 宏的值
- 该字符串位于只读数据段（`.rodata`），不可被修改
- 在进程整个生命周期内保持不变

### 不变量

`__libc_version` 的内容在程序启动后永不变更。它的地址（`&__libc_version`）可被安全地跨模块引用。

### Rust 实现注意事项

在 Rust `#![no_std]` 实现中：
- 可使用编译时环境变量或 `env!("CARGO_PKG_VERSION")` 替代 `VERSION` 宏
- 对应 Rust 定义：`#[no_mangle] static __libc_version: &[u8]` 或以 NUL 结尾的字节数组
- 若 rusl 不需要暴露此符号给 C 调用者，可使用 `#[used]` 防止链接器优化掉