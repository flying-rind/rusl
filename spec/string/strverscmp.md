# strverscmp

## 函数签名
```c
int strverscmp(const char *l0, const char *r0);
```

## 意图
比较两个字符串 l0 和 r0 的"版本号顺序"（GNU 风格）。与 strcmp 类似，但以更自然的方式处理数字序列（如 "file1" < "file10"）。

## 前置条件
- `l0 != NULL`
- `r0 != NULL`
- l0 和 r0 指向以 null 结尾的有效 C 字符串

## 后置条件
- 返回值 == 0：两个字符串按版本号规则相等
- 返回值 < 0：l0 在 r0 之前
- 返回值 > 0：l0 在 r0 之后
- l0 和 r0 指向的字符串内容不变

## 不变量
- 指针 i 标记第一个不同字符位置，dp 标记最长匹配前缀中最后非数字字符后一个位置
- z 标记最长匹配前缀中的数字后缀是否全为零

## 算法
1. 找到最长匹配前缀，跟踪 dp（最后一个非数字字符位置 + 1）和 z（数字序列是否全零）
2. 若首个不同字符位置处 l[dp] 和 r[dp] 均为数字：
   - 以数值方式比较数字序列长度（更长的数字串更大，除非前导零）
3. 若最长匹配前缀的数字后缀全为零且后续存在数字：
   - 数字排序在非数字之前
4. 否则：直接按字节比较 l[i] 和 r[i]

/* Rely */
[RELY]
Predefined Structures/Functions:
  int isdigit(int);                              // 依赖1: 判断字符是否为十进制数字，定义于 <ctype.h>
  size_t;                                        // 依赖2: 无符号整数类型，定义于 <stddef.h>

Predefined Macros:
  _GNU_SOURCE                                    // 依赖3: 启用 GNU 扩展函数 strverscmp 声明

[GUARANTEE]
Exported Interface:
  int strverscmp(const char *l0, const char *r0);  // 本模块保证对外提供的接口签名
