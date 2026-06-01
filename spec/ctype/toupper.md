# toupper 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int toupper(int c);
int __toupper_l(int c, locale_t l);
int toupper_l(int c, locale_t l);  // weak_alias -> __toupper_l
```

### 导出状态

| 符号 | 声明位置 | 导出状态 |
|------|---------|---------|
| `toupper` | `<ctype.h>` | **[Visibility]: Exported (ISO C 标准库)** |
| `__toupper_l` | musl 内部 | **[Visibility]: Internal (不导出)** -- `__` 前缀内部实现 |
| `toupper_l` | `<ctype.h>` (POSIX.1-2008) | **[Visibility]: Exported (POSIX 扩展)** -- `weak_alias` 弱符号 |

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是小写字母（`'a'`-`'z'`）
  - 返回对应的大写字母（`c & 0x5f`，等价于 `c - 32`）。
- Case 2: `c` 不是小写字母
  - 返回原值 `c`。

**[Post-condition]: `toupper_l` / `__toupper_l`**
- `__toupper_l(int c, locale_t l)` 忽略 `l` 参数，直接调用 `toupper(c)`。
- `toupper_l` 通过 `weak_alias(__toupper_l, toupper_l)` 绑定，是 POSIX 的 locale-aware 版本接口，行为与 `toupper` 相同（当前 musl 单 locale 实现）。

### 不变量

**[Invariant]:** 纯函数（无副作用，仅依赖输入参数）。线程安全（无共享可变状态）。

### 意图

将小写字母转换为大写字母。内部使用 `c & 0x5f` 位运算技巧（清除 bit5）实现高效转换。对于非小写字母，原样返回不做修改。

### 系统算法

```
if (islower(c)) return c & 0x5f;
return c;
```
先通过 `islower` 判断字符是否为小写字母，若是则清除 bit5（`& 0x5f`）完成大写转换。
ASCII 编码中小写字母范围 `0x61`-`0x7a`，对应大写 `0x41`-`0x5a`，相差 `0x20`（bit5）。
时间复杂度 O(1)，空间复杂度 O(1)。

---

## 依赖关系

| 调用者 | 被调用者 | 关系 |
|--------|---------|------|
| `toupper` | `islower` | 直接调用 |
| `__toupper_l` | `toupper` | 直接调用（忽略 locale 参数） |
| `toupper_l` | `__toupper_l` | `weak_alias` 别名（POSIX 导出） |

---

## 相关文件

- `src/ctype/toupper.c` — 实现文件
- `src/ctype/islower.c` — `islower` 实现
- `include/ctype.h` — 公共头文件声明

---

# Model: deepseek-v4-pro
