# towlower / towupper / casemap 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <wctype.h>

wint_t towlower(wint_t wc);
wint_t towupper(wint_t wc);
wint_t __towupper_l(wint_t c, locale_t l);
wint_t __towlower_l(wint_t c, locale_t l);
wint_t towupper_l(wint_t c, locale_t l);  // weak_alias
wint_t towlower_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
### towlower
- Case 1: `wc` 存在小写映射 → 返回对应的小写字符。
- Case 2: `wc` 不存在小写映射或 `wc == WEOF` → 返回 `wc` 本身。

### towupper
- Case 1: `wc` 存在大写映射 → 返回对应的大写字符。
- Case 2: `wc` 不存在大写映射或 `wc == WEOF` → 返回 `wc` 本身。

### 不变量

**[Invariant]:** - `tab`、`rulebases`、`rules`、`exceptions` 静态数组（来自 `casemap.h`）为编译时常量，程序生命周期内只读。
- `casemap` 为纯函数，无副作用。
- `towlower` 和 `towupper` 仅通过 `dir` 参数（0/1）区分映射方向。

### 意图

实现完整的 Unicode 大小写映射。`towlower` 将字符转换为小写，`towupper` 将字符转换为大写。共享 `casemap` 核心引擎，通过 `dir` 参数控制映射方向。支持简单 delta 映射和基于异常表的复杂映射。

### 系统算法

```
casemap(c, dir):
  Phase 1: c >= 0x20000 → 直接返回 c（CJK Extension B 及以上无大小写）
  Phase 2: 使用二级基-6 表将字符分为 6 种类型
  Phase 3: 根据规则类型和数据执行映射
  Phase 4: rt < 2 → 简单 delta 映射（return c0 + (rd & -(rt ^ dir))）
  Phase 5: 复杂映射时在 exceptions 数组中二分查找
时间复杂度: O(log n) 最坏情况，通常 O(1)。
```
