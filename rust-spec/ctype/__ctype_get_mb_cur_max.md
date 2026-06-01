# __ctype_get_mb_cur_max 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```rust
extern "C" fn __ctype_get_mb_cur_max() -> usize;
```

### 前置/后置条件

**[Pre-condition]:**
无前置条件。可在任何时刻调用。

**[Post-condition]:**
- Case 1: 当前 locale 的 `LC_CTYPE` 类别对应的 codeset 为 UTF-8
  - 返回 4。
- Case 2: 当前 locale 为 C / POSIX locale 或其他单字节编码
  - 返回 1。
- 返回值表示当前 locale 下多字节字符的最大字节数。

### 不变量

**[Invariant]:** 返回值仅依赖于当前线程的 locale 设置。在 locale 未变更期间，多次调用返回相同值。

### 意图

提供对 `MB_CUR_MAX` 宏的函数级实现。`<stdlib.h>` 中 `MB_CUR_MAX` 宏展开为 `__ctype_get_mb_cur_max()` 调用。在 Rust 内部实现中，可以通过访问线程本地 locale 结构判断当前是否为 UTF-8 locale。

### 系统算法

```
读取当前线程 locale 结构中的 LC_CTYPE 类别指针。
若设置了 cat[LC_CTYPE]（即 locale 非 C locale），返回 4（UTF-8）；
否则返回 1（单字节编码）。
时间复杂度 O(1)。
```