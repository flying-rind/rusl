# nonspacing.h 规约

> musl libc 内部 Unicode 非间距字符（组合标记）属性查找表。用于 `iswcntrl` 等函数识别不可打印的组合字符。

---

## 数据结构

### 查表数组

```c
static const unsigned char nonspacing[] = { ... };
```

[Visibility]: Internal — musl ctype 内部查找表，编译时嵌入 `.rodata`，不对外导出

#### Intent

存储 Unicode 字符的 "是否为非间距字符 (nonspacing mark, Mn/Mc/Me)" 属性。非间距字符（组合标记）包括 Unicode 通用类别 `Mn`（Non-Spacing Mark）、`Mc`（Spacing Combining Mark）、`Me`（Enclosing Mark），这类字符在渲染时与前一字符组合显示，通常不独立占用列宽。

#### 编码方式

- 两级索引：第一级通过码点高位字节定位段，第二级通过低位字节获取属性
- 值 `16` — 非组合标记（普通字符）
- 值 ≥ `18` — 编码了字符的类型标签
- 前半部分（0-255 字节区间）为 `unsigned char` 类型标签映射
- 后半部分为宽字符属性位图（与 `wide.h` 类似结构）

#### 使用方

- `wcwidth.c` — 零宽字符判断
- `iswcntrl.c` — 控制字符判定
- 其他宽字符分类函数

#### 存储结构

- 静态只读数据段（`.rodata`）
- 92 行，覆盖 BMP 范围

#### 依赖

无。
