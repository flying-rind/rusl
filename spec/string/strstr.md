# strstr

## 函数签名
```c
char *strstr(const char *h, const char *n);
```

## 意图
在字符串 h（haystack）中查找子串 n（needle）第一次出现的位置。

## 前置条件
- `h != NULL`
- `n != NULL`
- h 和 n 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若 n[0] == '\0'，返回值为 (char*)h
- 若在 h 中找到 n，返回值为指向 h 中第一个匹配位置的指针
- 若未找到，返回值为 NULL
- h 和 n 指向的字符串内容不变

## 不变量
- 搜索使用递增的 h 指针，从不回溯超过已检查范围
- Two-Way 算法中 shift 表、byteset 在搜索期间不变

## 算法
1. 若 n 为空串，返回 h
2. 使用 strchr 跳跃到 n 首字符在 h 中首次出现的位置
3. 根据 n 长度选择子算法：
   - l==1: 已被 strchr 完成
   - l==2: twobyte_strstr（16 位滑动窗口）
   - l==3: threebyte_strstr（32 位滑动窗口）
   - l==4: fourbyte_strstr（32 位滑动窗口）
   - l>=5: twoway_strstr（Two-Way 算法）

### twoway_strstr 算法细节
- 计算 needle 长度的同时填充 shift 表与 byteset
- 在 haystack 中逐渐扩展结束边界 z（使用 memchr 查找 '\0'）
- 使用坏字符规则跳过不匹配位置
- 使用最大后缀分解进行双向比较

/* Rely */
[RELY]
Predefined Structures/Functions:
  char *strchr(const char *, int);              // 依赖1: 查找字符首次出现，定义于 <string.h>
  void *memchr(const void *, int, size_t);      // 依赖2: 在内存区域中查找字符，定义于 <string.h>
  int memcmp(const void *, const void *, size_t);  // 依赖3: 比较内存区域，定义于 <string.h>
  uint16_t;                                      // 依赖4: 16位无符号整数类型，定义于 <stdint.h>
  uint32_t;                                      // 依赖5: 32位无符号整数类型，定义于 <stdint.h>
  size_t;                                        // 依赖6: 无符号整数类型，定义于 <stddef.h>

Predefined Macros:
  (none)

[GUARANTEE]
Exported Interface:
  char *strstr(const char *h, const char *n);  // 本模块保证对外提供的接口签名
