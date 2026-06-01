# memmem

## 函数签名
```c
void *memmem(const void *h0, size_t k, const void *n0, size_t l);
```

## 意图
在长度为 k 的内存区域（haystack）h0 中查找长度为 l 的子序列（needle）n0 第一次出现的位置。

## 前置条件
- `h0 != NULL` 或 `k == 0`
- `n0 != NULL` 或 `l == 0`
- 当 `k > 0` 时，`h0` 指向的内存区域至少可读取 k 个字节
- 当 `l > 0` 时，`n0` 指向的内存区域至少可读取 l 个字节

## 后置条件
- 若 l == 0，返回 (void*)h0
- 若 k < l，返回 NULL
- 若找到匹配的子序列，返回指向 h0 中第一次匹配位置的指针
- 若未找到，返回 NULL

## 不变量
- 搜索窗口始终在 h0[0..k-1] 区间内
- 两路算法中 shift 表、byteset 等辅助数据结构在搜索期间不变

## 算法
1. 若 l == 0，返回 h0
2. 若 k < l，返回 NULL
3. 使用 memchr 跳转到 needle 首字节在 haystack 中首次出现的位置
4. 根据 needle 长度选择算法：
   - l==1: 已由 memchr 完成
   - l==2: twobyte_memmem（16 位滑动窗口）
   - l==3: threebyte_memmem（32 位滑动窗口）
   - l==4: fourbyte_memmem（32 位滑动窗口）
   - l>=5: twoway_memmem（Two-Way 字符串匹配算法，含坏字符规则和好前缀规则，O(n) 时间复杂度）

### twoway_memmem 算法细节
- 计算 needle 的最大后缀及其周期 p
- 构建 byteset 位图用于快速检查特定字节是否属于 needle
- 构建 shift 表用于坏字符规则跳转
- 从左到右比较左半部分，从右到左比较右半部分
- 若 needle 为周期串（mem0 = l-p），则利用周期性加速跳转

/* Rely */
[RELY]
Predefined Structures/Functions:
  void *memchr(const void *src, int c, size_t n);  // 依赖1: 在 haystack 中查找 needle 首字节首次出现位置
  int memcmp(const void *vl, const void *vr, size_t n);  // 依赖2: 在 twoway_memmem 中判断 needle 是否为周期串
  size_t;  // 依赖3: 无符号整数类型，用于索引、计数和位图操作
  uint16_t;  // 依赖4: 16 位无符号整数类型，用于 twobyte_memmem 的 16 位滑动窗口匹配
  uint32_t;  // 依赖5: 32 位无符号整数类型，用于 threebyte/fourbyte_memmem 的 32 位滑动窗口匹配
  unsigned char;  // 依赖6: 单字节无符号类型，用于字节级 needle/haystack 元素访问

Predefined Macros:
  _GNU_SOURCE  // 依赖7: 启用 GNU 扩展函数声明（memmem 是 GNU 扩展，非 POSIX 标准）
  MAX(a,b)  // 依赖8: 取两值较大者，用于计算搜索步长和记忆边界
  MIN(a,b)  // 依赖9: 取两值较小者，用于计算跳转步长
  BITOP(a,b,op)  // 依赖10: 位图操作宏，用于 byteset 的快速字节成员检测

[GUARANTEE]
Exported Interface:
  void *memmem(const void *h0, size_t k, const void *n0, size_t l);  // 在 h0 中查找子串 n0 首次出现位置（长度 k 的 haystack 中查找长度 l 的 needle）
