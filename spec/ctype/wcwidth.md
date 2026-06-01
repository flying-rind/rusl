# wcwidth 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <wchar.h>

static const unsigned char table[] = {
#include "nonspacing.h"
};

static const unsigned char wtable[] = {
#include "wide.h"
};

int wcwidth(wchar_t wc);
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wchar_t`，Unicode 码点值。

**[Post-condition]:**
- Case 1: `wc` 是 null 字符（L'\0'）→ 返回 0。
- Case 2: `wc` 是可打印字符
  - `wc < 0xff` 且 `(wc+1 & 0x7f) >= 0x21`：返回 1
  - BMP 普通宽度字符：返回 1
  - 宽字符（CJK 等，在 `wtable` 位图中）：返回 2
- Case 3: `wc` 是组合字符（nonspacing mark，在 `table` 位图中）→ 返回 0。
- Case 4: `wc` 是不可打印字符
  - `wc` 在 0x01-0x1F 或 0x7F-0x9F 范围内（C0/C1 控制字符）：返回 -1
  - `(wc & 0xfffe) == 0xfffe`（非字符码点）：返回 -1
  - U+E0001 或 U+E0020-U+E00EF 范围内的标记字符：返回 0

### 不变量

**[Invariant]:** - `table`（nonspacing.h）和 `wtable`（wide.h）为编译时常量，运行时只读。
- 函数为纯函数，线程安全。

### 意图

确定宽字符在终端显示时占用的列数。返回 0（组合字符）、1（普通字符）、2（宽字符如 CJK）或 -1（不可打印）。用于终端仿真器和格式化工具。

### 系统算法

```
Phase 1（快速路径 - ASCII 范围）:
  wc < 0xff:
    - (wc+1 & 0x7f) >= 0x21 → 可打印 → 返回 1
    - wc == 0 → 返回 0（null）
    - 其余 → 返回 -1（控制字符）

Phase 2（BMP 字符分类）:
  (wc & 0xfffeffff) < 0xfffe:
    - 查 nonspacing 位图 → 命中 → 返回 0
    - 查 wide 位图 → 命中 → 返回 2
    - 其余 → 返回 1

Phase 3（高位平面处理）:
  - (wc & 0xfffe) == 0xfffe → 返回 -1（非字符码点）
  - wc 在 SIP 范围（0x20000-0x3FFFF）→ 返回 2
  - wc == 0xe0001 或 wc 在 U+E0020-U+E00EF 范围 → 返回 0
  - 其余 → 返回 1
```
