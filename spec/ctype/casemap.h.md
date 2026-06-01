# casemap.h 规约

> musl libc 内部 Unicode 大小写映射数据表。定义 `towctrans`/`towupper`/`towlower` 等函数所需的字符大小写转换规则。

---

## 数据结构

### 1. 状态转移表 `tab[]`

```c
static const unsigned char tab[] = { ... };
```

[Visibility]: Internal — musl ctype 内部查找表，编译时嵌入 `.rodata`，不对外导出

#### Intent

存储 Unicode 字符大小写转换的状态机转移表。根据字符的输入类型标签（由 `alpha.h` 等分类表获得），查表得到对应的转换规则组索引，随后在 `rules[]` 表中解析具体的大小写映射。

#### 编码方式

- 每个字节对应一个 Unicode 位段的类型标签映射值（0-255）
- 值 `0` — 不受大小写转换影响
- 值 `6` — 默认类型（无特殊大小写规则）
- 其他值 — 索引到 `rulebases[]` 以定位对应的转换规则组
- 169 行数据，覆盖 Unicode BMP 范围

### 2. 转换规则表 `rules[]`

```c
static const int rules[] = { ... };
```

[Visibility]: Internal — musl ctype 内部规则表，不对外导出

#### Intent

存储 Unicode 字符的大小写转换规则。每条规则编码一个字符的 Unicode 码点偏移量（大小写差），或指示特殊的大小写转换逻辑（如标题转换为多个字符）。

#### 编码方式

- 每个 `int` (32位) 编码一条规则
- 正值表示码点加上此值得大写/小写形式
- 负值表示码点减去此绝对值得大写/小写形式
- 高字节可能编码特殊标记（如上下文相关的大小写转换）
- `0x0` — 终止标记

### 3. 规则基址表 `rulebases[]`

```c
static const unsigned char rulebases[] = { ... };
```

[Visibility]: Internal — musl ctype 内部索引表，不对外导出

#### Intent

从 `tab[]` 获得的类型标签映射到 `rules[]` 中的规则组起始索引。

#### 编码方式

- 每个字节为 `rules[]` 数组的起始偏移索引
- 值 `0` 表示该类型无转换规则

### 4. 异常表 `exceptions[][]`

```c
static const unsigned char exceptions[][2] = { ... };
```

[Visibility]: Internal — musl ctype 内部异常表，不对外导出

#### Intent

存储规则的例外情况：某些字符的大小写转换结果不是简单的单字符码点偏移，而是需要映射到完全不同的字符（如 `ß` → `SS` 的标题转换，或土耳其语 locale 的 `İ`/`ı` 映射）。

#### 编码方式

- 每行 2 个 `unsigned char`：`{ 输入类型标签, 输出类型标签 }`
- 用于覆写 `tab[]` 的标准映射

### 使用方

- `towctrans.c` — 宽字符大小写转换
- `towupper.c` / `towlower.c` — 大小写转换
- `__ctype_toupper_loc.c` / `__ctype_tolower_loc.c` — 单字节大小写映射表初始化

### 存储结构

- 所有表均为静态只读数据（`.rodata`）
- 总计约 500 字节

### 依赖

无。
