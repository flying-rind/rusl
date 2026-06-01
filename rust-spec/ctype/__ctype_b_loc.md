# __ctype_b_loc 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```rust
use core::ffi::c_ushort;

extern "C" fn __ctype_b_loc() -> *const *const c_ushort;
```

### 前置/后置条件

**[Pre-condition]:**
无前置条件。

**[Post-condition]:**
- 返回指向字符分类表的指针的指针。解引用一次得到表指针 `table_ptr`，该表有 384 个 `c_ushort` 元素，索引偏移量为 +128（即 `table_ptr[-128]` 到 `table_ptr[255]` 有效）。
- 返回的指针在整个程序生命周期内有效，指向只读数据段。
- 表内容按字节序（大端/小端）自动调整：`table` 中的值通过预处理宏 `X(x)` 进行字节交换，确保小端平台上运行时内存布局与预期一致。

### 不变量

**[Invariant]:** 纯函数，始终返回同一常量指针。无内部可变状态。表内容永不改变。

### 意图

返回 C locale 字符分类位掩码表的地址。该表被 `<ctype.h>` 中的宏（如 `isalpha(c)`）用于 O(1) 字符分类查询。表中每个元素的位掩码含义由 `alpha.h` 中的位定义确定。

在 Rust 内部实现中，可使用 `static` 不可变数组存储分类表，利用 `#[cfg(target_endian = "little/big")]` 处理字节序差异。

### 系统算法

```
返回内部静态表指针的地址。表组织方式：
- table[0..127]:   索引 -128 到 -1 的条目（全为零，实际不使用）
- table[128..255]: 索引 0 到 127 的条目（ASCII 范围）
- table[256..383]: 索引 128 到 255 的条目（扩展 ASCII）
ptable = table + 128，返回 &ptable。
时间复杂度 O(1)。
```