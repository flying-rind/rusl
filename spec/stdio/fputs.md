# fputs.c 规约

> musl libc 标准 IO 字符串输出实现。将 C 字符串写入 FILE 流（不追加换行符）。

---

## 依赖图

```
fputs (Public)
  ├── strlen (from <string.h>)
  ├── fwrite (see fwrite.c spec)
  └── weak_alias → fputs_unlocked
```

---

## 函数规约

### 1. fputs

```c
int fputs(const char *restrict s, FILE *restrict f);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

将以 `\0` 结尾的 C 字符串 `s` 写入 FILE 流 `f`（不包括结尾的 `\0`，不自动追加换行符）。通过 `fwrite` 完成实际写入，返回非负值表示成功，`EOF` 表示失败。

#### 前置条件

- `s`: 非空指针，指向以 `\0` 结尾的有效 C 字符串
- `f`: 非空 FILE 指针，指向已打开的写模式流

#### 后置条件

- **Case 1 成功写入完整字符串**
  - 返回非负值（具体为 0 或正值，musl 实现中若写入成功返回 0）
  - 字符串内容已写入流

- **Case 2 写入失败**
  - 返回 `EOF`（通常为 -1）
  - `errno` 可能被设置

#### 系统算法

```
fputs(s, f):
  l = strlen(s)                         // 获取字符串长度(不含 \0)
  return (fwrite(s, 1, l, f) == l) - 1  // 全部写入成功则返回 0，否则返回 -1 (EOF)
```

**返回值技巧说明**: 表达式 `(fwrite(...) == l) - 1` 利用 C 语言布尔值到整数的隐式转换：
- 若 `fwrite` 返回 `l`（全部写入成功），`==` 得 1，`1 - 1 = 0` → 非负返回
- 若 `fwrite` 返回 `< l`（部分写入或错误），`==` 得 0，`0 - 1 = -1` → `EOF`

#### 不变量

- `fputs` 本身不添加换行符（与 `puts` 的区别）

#### 依赖

- `strlen` — 计算字符串长度（`<string.h>`）
- `fwrite(const void *, size_t, size_t, FILE *)` — 块写入函数（定义于 `fwrite.c`）

---

### 2. fputs_unlocked (weak_alias)

```c
// weak_alias(fputs, fputs_unlocked);
int fputs_unlocked(const char *restrict s, FILE *restrict f);
```

[Visibility]: User — POSIX 免锁 `fputs`，在 musl 中与 `fputs` 共享同一实现

前置/后置条件及行为：完全等同于 `fputs`。
