# strcasecmp

## 函数签名
```c
int strcasecmp(const char *_l, const char *_r);
```

## 意图
在忽略大小写的情况下比较两个 C 字符串 _l 和 _r。

## 前置条件
- `_l != NULL`
- `_r != NULL`
- _l 和 _r 指向以 null 结尾的有效 C 字符串

## 后置条件
- 返回值 == 0：两个字符串忽略大小写后完全相等（同时到达 null 终止符）
- 返回值 < 0：在第一个不同字符处 tolower(_l[i]) < tolower(_r[i])
- 返回值 > 0：在第一个不同字符处 tolower(_l[i]) > tolower(_r[i])
- _l 和 _r 指向的字符串内容不变

## 不变量
- 循环在字符串终止符 '\0' 或字符不相等时退出
- 每次比较都通过 tolower() 转换为小写后再比较

## 算法
逐字符比较，每个字符经过 tolower() 转换为小写后进行比较。循环继续条件是两字符均为非 '\0' 且相等（原始或小写后相等）。返回值是两个小写字符的差值。

/* Rely */
[RELY]
Predefined Structures/Functions:
  int tolower(int c);  // 依赖1: 将字符转换为小写，忽略大小写比较的核心
  locale_t;            // 依赖2: 区域设置类型，用于 __strcasecmp_l 参数
  int;                 // 依赖3: 返回值和字符比较的基础类型
  unsigned char;       // 依赖4: 用于无符号字符比较，防止负数 char 的符号扩展问题
  void *;              // 依赖5: 指针转换，用于将 const char* 转为 const unsigned char*
  NULL;                // 依赖6: 空指针常量

Predefined Macros:
  weak_alias(__strcasecmp_l, strcasecmp_l);  // 内部宏: 将 __strcasecmp_l 导出为弱符号 strcasecmp_l

[GUARANTEE]
Exported Interface:
  int strcasecmp(const char *_l, const char *_r);          // 忽略大小写的字符串比较
  int __strcasecmp_l(const char *l, const char *r, locale_t loc);  // 内部实现，可接受 locale 参数
  int strcasecmp_l(const char *l, const char *r, locale_t loc);    // 弱别名，locale 感知的公开接口
