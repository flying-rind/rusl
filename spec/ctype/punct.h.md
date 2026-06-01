# punct.h 规约

> musl libc 内部 Unicode 标点符号属性查找表。用于 `iswpunct` 等函数快速判断字符是否为标点符号。

---

## 数据结构

### 查表数组

```c
static const unsigned char punct[] = { ... };
```

[Visibility]: Internal — musl ctype 内部查找表，编译时嵌入 `.rodata`，不对外导出

#### Intent

存储 Unicode 字符的 "是否为标点符号 (punctuation)" 属性。标点符号包括 Unicode 通用类别 `Pc`（Connector）、`Pd`（Dash）、`Ps`（Open）、`Pe`（Close）、`Pi`（Initial quote）、`Pf`（Final quote）、`Po`（Other）以及数学符号 `Sm`、货币符号 `Sc`、修饰符号 `Sk`、其他符号 `So` 的部分子集。

#### 编码方式

- 两级索引：第一级通过码点高位字节定位段，第二级通过低位字节获取属性
- 值 `16` — 非标点字符
- 值 `17` — （保留/特殊用途）
- 值 ≥ `18` — 编码了字符的类型标签
- 前半部分（0-255 字节区间）为 `unsigned char` 类型标签映射
- 后半部分为宽字符属性位图

#### 使用方

- `iswpunct.c` — 宽字符标点判定
- 其他宽字符分类函数

#### 存储结构

- 静态只读数据段（`.rodata`）
- 141 行，覆盖 BMP 范围

#### 依赖

无。
