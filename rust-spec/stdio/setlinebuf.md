# setlinebuf 函数规约

## 复杂度分级: Level 1

> musl libc `setlinebuf` 实现 — GNU 扩展，将 FILE 流设为行缓冲模式。不提供用户缓冲区，musl 将根据实际使用情况自动分配内部缓冲区。

---

## 函数接口

```rust
extern "C" fn setlinebuf(f: *mut FILE);
```

[Visibility]: User — GNU / BSD 扩展函数（需定义 `_GNU_SOURCE` 或 `_BSD_SOURCE`），用户程序可直接调用。在 Rust 侧通过 `#[no_mangle] pub unsafe extern "C"` 导出，保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 有效的 `*mut FILE` 指针，指向已打开的流（非空）
- 根据 C 标准和实现惯例，`setlinebuf` 的行为仅在流打开后、任何读写操作前被调用时是定义良好的
- 编译时需定义 `_GNU_SOURCE` 或 `_BSD_SOURCE` 以获取函数声明

**[Post-condition]:**
- 等价于 `setvbuf(f, null_mut(), _IOLBF, 0)`：
  - 不提供用户缓冲区（`buf` = null），musl 在需要读/写时自动分配内部缓冲区
  - 设置为行缓冲模式：每当遇到换行符 `b'\n'` 时自动刷新输出缓冲区
- 流 `f` 的 `F_SVB` 标志被置位
- 无返回值（`void` → Rust 返回 `()`）

**[Error Behavior]:**
- 本函数不产生错误。`setvbuf` 不会因无效 `type_` 而失败（因传入的是合法值 `_IOLBF`）
- 不设置 errno

---

### 不变量

**[Invariant]:**
- `setlinebuf(f)` 始终等价于 `setvbuf(f, null_mut(), _IOLBF, 0)`
- 调用后 `F_SVB` 标志必然置位
- `f.lbf` 将在内部缓冲区分配后被设为 `b'\n'`

---

### 意图

将流 `f` 设为行缓冲模式。这是 musl 中 `setvbuf` 的最简单包装：不提供用户缓冲区，也不指定缓冲区大小，完全依赖 musl 内部自动分配。

Rust 侧实现：
- 内部直接委托给 `setvbuf`，传入 `null_mut()` 和 `0` 参数
- 无需显式参数类型导入（仅有 `*mut FILE`）
- `null_mut()` 来自 `core::ptr::null_mut`
- 因是 GNU 扩展，可放在 `#[cfg(feature = "gnu")]` 或类似的 feature gate 下

### 系统算法

```
setlinebuf(f):
  1. setvbuf(f, null_mut(), _IOLBF, 0)
  2. 返回  // void 函数
```

时间复杂度 O(1)（取决于 `setvbuf` 的 O(1) 复杂度）。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `_IOLBF` | 1 | `<stdio.h>` | 行缓冲模式 |

---

## 依赖图

```
setlinebuf (Public, GNU extension)
  └── setvbuf (see setvbuf spec)
```

---

## [RELY]

- `setvbuf(f: *mut FILE, buf: *mut c_char, type_: c_int, size: size_t) -> c_int` — 底层的缓冲设置函数（见 `setvbuf` spec）
- 常量 `_IOLBF`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn setlinebuf(f: *mut FILE);`

本模块保证对外提供 ABI 兼容的 `setlinebuf` 函数符号，行为符合 GNU/BSD 扩展语义。`setlinebuf(f)` 始终等价于 `setvbuf(f, NULL, _IOLBF, 0)`。
