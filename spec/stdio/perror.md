# perror.c 规约

> musl libc 标准库错误信息打印函数实现。向 stderr 输出与当前 errno 对应的错误消息。

---

## 依赖图

```
perror (Public)
  ├── stderr                                  — 标准错误流 (全局变量, src/stdio/__stderr_used.c)
  ├── strerror(errno)                         — 获取 errno 对应的错误字符串 (<string.h>)
  ├── strlen(msg) / strlen(errstr)            — 计算字符串长度 (<string.h>)
  ├── fwrite(msg, strlen(msg), 1, f)          — 写入用户消息 (src/stdio/fwrite.c)
  ├── fputc(':', f) / fputc(' ', f)           — 写入分隔符 (src/stdio/fputc.c)
  ├── fwrite(errstr, strlen(errstr), 1, f)    — 写入错误描述 (src/stdio/fwrite.c)
  ├── fputc('\n', f)                          — 写入换行符 (src/stdio/fputc.c)
  └── FLOCK(f) / FUNLOCK(f)                   — 流锁定/解锁 (stdio_impl.h)
```

---

## 函数规约

### 1. perror

```c
void perror(const char *msg);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

向 `stderr` 输出描述最后一个系统调用错误的字符串。输出格式为：若 `msg` 非空且非空字符串，则输出 `"<msg>: <error_message>\n"`；否则仅输出 `"<error_message>\n"`。错误消息由 `strerror(errno)` 获得。

关键实现细节：在输出前后保存和恢复 `stderr` 的 locale 和 orientation（mode）字段，因为 POSIX 规定 `perror` 不能改变流的这些属性。

#### 前置条件

- `msg`: 可为 `NULL`（表示不输出前缀）；若非 `NULL`，指向以 `\0` 结尾的字符串
- `stderr` 已初始化且可写
- `errno` 包含一个有效错误码（由调用之前的错误系统调用设置）

#### 后置条件

- **Case 1 输出成功**
  - 错误消息已写入 `stderr`
  - `stderr` 的 locale 和 mode 保持不变
  - `errno` 可能被 `fwrite`/`fputc` 修改

- **Case 2 输出失败**
  - 部分或全部消息写入失败
  - `stderr` 的 locale 和 mode 保持不变
  - 不检查 `fwrite`/`fputc` 返回值（perror 本身无返回值）

#### 系统算法

```
perror(msg):
  1. f = stderr
  2. errstr = strerror(errno)       // 获取错误描述字符串
  
  3. FLOCK(f)                       // 锁定 stderr
  
  4. 保存上下文:
     old_locale = f->locale         // 保存 locale 设置
     old_mode = f->mode             // 保存方向/编码
  
  5. 输出:
     if msg && *msg:                // msg 非空且非空字符串
       fwrite(msg, strlen(msg), 1, f)  // 写入用户消息
       fputc(':', f)                   // 写入冒号分隔符
       fputc(' ', f)                   // 写入空格
     fwrite(errstr, strlen(errstr), 1, f)  // 写入系统错误描述
     fputc('\n', f)                     // 写入换行符
  
  6. 恢复上下文:
     f->mode   = old_mode           // 恢复方向/编码
     f->locale = old_locale         // 恢复 locale
  
  7. FUNLOCK(f)                     // 解锁 stderr
```

#### 不变量

- `perror` 不会改变 `stderr` 的 orientation（字节/宽字符方向）和 locale 设置
- 输出始终以换行符结束
- `perror` 无返回值，不报告 I/O 错误

#### 依赖

- `stderr` — 标准错误流 FILE 指针（全局变量，定义于 `src/stdio/__stderr_used.c`）
- `strerror(int errnum)` — 获取错误号的描述字符串（来自 `<string.h>`）
- `strlen(const char *s)` — 计算字符串长度（来自 `<string.h>`）
- `fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream)` — 块写入（定义于 `src/stdio/fwrite.c`）
- `fputc(int c, FILE *stream)` — 写入单个字符（定义于 `src/stdio/fputc.c`）
- `FLOCK(FILE *f)` / `FUNLOCK(FILE *f)` — 获取/释放 FILE 锁（宏，定义于 `src/internal/stdio_impl.h`）
- `errno` — 当前线程错误码（定义于 `src/errno/__errno_location.c`）
- `FILE` 结构体 `locale`、`mode` 字段（定义于 `src/internal/stdio_impl.h`）

#### 输出格式示例

| msg | errno | 输出 |
|-----|-------|------|
| `"open"` | `ENOENT` | `open: No such file or directory\n` |
| `NULL` | `EACCES` | `Permission denied\n` |
| `""` | `EINVAL` | `Invalid argument\n` |
