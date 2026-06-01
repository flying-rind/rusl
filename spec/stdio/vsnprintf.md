# vsnprintf

## 函数签名
```c
int vsnprintf(char *restrict s, size_t n, const char *restrict fmt, va_list ap);
```

## 内部函数
```c
static size_t sn_write(FILE *f, const unsigned char *s, size_t l);
```

## 内部结构体
```c
struct cookie { char *s; size_t n; };
```

---

## sn_write（内部回调）

### 意图
作为 `FILE` 的输出回调，将格式化数据写入 `cookie` 管理的用户缓冲区，自动截断超出 `n` 限制的数据。

### 前置条件
- `f->cookie` 指向有效的 `struct cookie`
- `f->wbase` / `f->wpos` 指向 `FILE` 内部缓冲区间

### 后置条件
- 最多 `cookie->n` 字节被拷贝到 `cookie->s`
- `cookie->s` 末尾写入 `'\0'`
- 返回 `l`（始终报告"成功写入全部数据"以维持 `vfprintf` 继续产出字符）

### 算法
1. 先冲刷 `FILE` 内部缓冲中已有的数据(从 `wbase` 到 `wpos` 区间)到 `cookie->s`，受 `cookie->n` 限制
2. 再将本次 `vfprintf` 产出的新数据 `s[0..l)` 拷贝到 `cookie->s`，受剩余 `cookie->n` 限制
3. 始终追加 `'\0'` 终止符
4. 重置 `FILE` 写指针，返回 `l`

---

## vsnprintf（主函数）

### 意图
将格式化字符串写入固定大小缓冲区，保证不溢出且始终 null 终止。行为等价于 `snprintf(s, n, fmt, ...)` 但接受 `va_list`。

### 前置条件
- `fmt != NULL`，指向有效格式化字符串
- `ap` 已由 `va_start` 正确初始化
- 若 `n > 0`，则 `s` 指向至少 `n` 字节的可写内存; 若 `n == 0`，则 `s` 可为 `NULL`

### 后置条件
- Case 1 正常: 返回值 `r` 为格式化完成后字符串的总长度（不含 `'\0'`），等价于"若缓冲区足够大时本应写入的字符数"
- Case 2 编码错误: 返回负值（由 `vfprintf` 传递）
- 若 `n > 0`，则 `s[0..min(r, n-1)]` 包含截断后的格式化输出，`s[min(r, n-1)] == '\0'`
- 若 `n == 0`，则 `s` 不会被写入

### 不变量
- `cookie.n` 始终是当前剩余可写字节数（不含尾部 `'\0'`）
- `cookie.s` 始终指向下一个待写入位置
- `FILE` 对象仅在栈上存在，无并发访问

### 算法
1. 若 `n == 0`，将输出导向 `dummy[1]` 哑缓冲区（副作用为返回值仍计算完整字符串长度）
2. 若 `n > 0`，预留 1 字节给 `'\0'`，即有效写入容量为 `n-1`
3. 构造 `FILE` 对象：锁定禁用(`lock=-1`)、缓冲为 `buf[1]`（无缓冲语义）、写回调为 `sn_write`
4. 调用 `vfprintf(&f, fmt, ap)`，将格式化结果逐块写入 `FILE`
5. 返回 `vfprintf` 的返回值

/* Rely */
[RELY]
Predefined Structures/Functions:
  struct FILE { ... };             // 依赖1: 标准 I/O 流结构体，定义于 stdio_impl.h
  int vfprintf(FILE *restrict f, const char *restrict fmt, va_list ap);
                                   // 依赖2: 核心格式化输出引擎
  void *memcpy(void *restrict dest, const void *restrict src, size_t n);
                                   // 依赖3: 内存拷贝
Predefined Macros:
  MIN(a, b)                        // 依赖4: 本地定义的最小值宏
  EOF                              // 依赖5: 行缓冲模式常量

[GUARANTEE]
Exported Interface:
  int vsnprintf(char *restrict s, size_t n, const char *restrict fmt, va_list ap);
                                   // 本模块保证对外提供 vsnprintf 接口