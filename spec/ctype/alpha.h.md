# alpha.h 规约

> musl libc 内部 Unicode 字母属性查找表。定义两级索引表 `alpha.h`，用于 `iswalpha` 等宽字符分类函数的 O(1) 查表实现。

---

## 数据结构

### 查表数组

```c
static const unsigned char alpha[] = { ... };
```

[Visibility]: Internal — musl ctype 内部查找表，编译时嵌入 `.rodata`，不对外导出

#### Intent

存储 Unicode 字符的 "是否为字母 (alphabetic)" 属性位图。对于每个 Unicode 码点，通过两级索引快速判断该字符类别。该表本身为一级表：使用区间编码，每个字节表示一个原类型 (canonical combining class 相关的类型值)，ctyp 函数通过查表获得该值后进一步判断。

#### 编码方式

- 索引：Unicode 码点 `c` 通过高位字节作为第一级索引，低位字节作为第二级偏移
- 值：每个字节表示字符的类型标签（如 `17` = 大写字母, `16` = 非字母等）
- 仅覆盖 BMP (Basic Multilingual Plane, U+0000 ~ U+FFFF)

#### 使用方

- `iswalpha.c` — 检查宽字符是否为字母
- `iswlower.c` / `iswupper.c` — 大小写分类
- 其他宽字符分类函数

#### 存储结构

- 静态只读数据段（`.rodata`)
- 173 行，覆盖 BMP 范围内的所有 Unicode 码点（65536 个）

#### 依赖

无。
