# ungetwc.c 规约

> musl libc `ungetwc` 实现 — 将一个宽字符推回 FILE 流的输入缓冲区。需要处理多字节编码转换和 locale 管理。

---

## 依赖图

```
ungetwc (Public)
  ├── FLOCK / FUNLOCK (锁宏, 定义于 stdio_impl.h)
  │     ├── __lockfile (see __lockfile.c spec)
  │     └── __unlockfile (see __unlockfile.c spec)
  ├── __toread (see __toread.c spec)
  ├── fwide (from <wchar.h>)
  ├── wcrtomb (from <wchar.h>)
  ├── memcpy (from <string.h>)
  ├── isascii (from <ctype.h>)
  └── CURRENT_LOCALE / f->locale (locale 管理)
```

---

## 函数规约

### 1. ungetwc

```c
wint_t ungetwc(wint_t c, FILE *f);
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用（ISO C 扩展）

#### Intent

将宽字符 `c` 推回 FILE 流 `f` 的读缓冲区。与 `ungetc` 的核心区别在于：

1. **宽字符支持**：推回的是 `wint_t`（宽字符类型），底层需转换为多字节序列存入字节缓冲区
2. **locale 感知**：必须使用流的 locale 进行宽字符到多字节的转换
3. **多字节序列**：宽字符 `c` 转换为多字节序列 `mbc`（长度最多 `MB_LEN_MAX`），然后推回

#### 前置条件

- `f`: 有效的 `FILE *` 指针，指向已打开的流
- `c`: 要推回的宽字符值（以 `wint_t` 传递），可以是 `WEOF` 以外的任何有效宽字符
- 流 `f` 必须处于读模式或有足够空间容纳推回字符的多字节表示
- 当前 locale 必须支持 `c` 的多字节编码

#### 后置条件

- Case 1 成功推回（ASCII 字符，`isascii(c)` 为真）:
  - 单字节 `(unsigned char)c` 被推回读缓冲区
  - `f->rpos` 递减 1
  - 流的 `F_EOF` 标志被清除
  - locale 恢复为调用前的值
  - 返回 `c`

- Case 2 成功推回（非 ASCII 多字节字符）:
  - 宽字符 `c` 通过 `wcrtomb` 转换为多字节序列 `mbc`（长度 `l`）
  - 多字节序列通过 `memcpy` 写入缓冲区（`f->rpos` 递减 `l`）
  - 流的 `F_EOF` 标志被清除
  - locale 恢复为调用前的值
  - 返回 `c`

- Case 3 失败（任一下列条件）:
  - `c == WEOF`
  - 流无法进入读模式（`__toread` 后 `rpos` 仍为 NULL）
  - `wcrtomb` 转换失败（返回负值）
  - 推回空间不足（`f->rpos < f->buf - UNGET + l`）
  - locale 恢复为调用前的值
  - 返回 `WEOF`

#### 系统算法

```
ungetwc(c, f):
  1. 保存当前 locale: loc = CURRENT_LOCALE
  2. FLOCK(f) — 获取流锁
  3. 若 f->mode <= 0，调用 fwide(f, 1) 设置为宽字符模式
  4. 切换到流的 locale: CURRENT_LOCALE = f->locale
  5. 若 f->rpos 为 NULL，调用 __toread(f) 初始化读模式
  6. 若以下任一条件成立，失败:
       - f->rpos == NULL（无法进入读模式）
       - c == WEOF
       - wcrtomb(mbc, c, 0) 返回 < 0（转换失败）
       - f->rpos < f->buf - UNGET + l（空间不足）
     → 释放锁，恢复 locale，返回 WEOF
  7. 若 isascii(c): 单字节推回 *--f->rpos = c
     否则: memcpy(f->rpos -= l, mbc, l) 多字节推回
  8. f->flags &= ~F_EOF — 清除 EOF 标志
  9. FUNLOCK(f) — 释放锁
  10. 恢复 locale: CURRENT_LOCALE = loc
  11. 返回 c
```

#### Intent 关键设计点

- **locale 安全**：函数在入口保存当前 locale，在流的 locale 上下文中执行宽字符转换，最后在返回前恢复原始 locale。这确保了 locale 的线程安全性
- **多字节推回空间检查**：需要确保推回的多字节序列（长度 `l`）加上 `UNGET` 预留空间不超出缓冲区边界
- **宽字符模式**：若流尚未设置方向（`mode <= 0`），通过 `fwide(f, 1)` 将其设置为宽字符模式

#### 不变量

- 无论成功或失败，调用前后的 `CURRENT_LOCALE` 值必须一致
- 推回多字节序列时，字节顺序与 `wcrtomb` 输出一致（即保持多字节序列的正确编码）

#### 依赖

- `__toread(FILE *)` — 将流切换到读模式（定义于 `src/stdio/__toread.c`）
- `__lockfile(FILE *)` / `__unlockfile(FILE *)` — 流加锁/解锁（定义于 `src/stdio/__lockfile.c`）
- `fwide(FILE *, int)` — 设置 / 查询流宽窄模式（定义于 `<wchar.h>`）
- `wcrtomb(char *, wchar_t, mbstate_t *)` — 宽字符到多字节转换（定义于 `<wchar.h>`）
- `memcpy(void *, const void *, size_t)` — 内存拷贝（定义于 `<string.h>`）
- `isascii(int)` — ASCII 字符检测宏（定义于 `<ctype.h>`）
- `CURRENT_LOCALE` / locale 管理 — 线程 locale 切换（定义于 `locale_impl.h`）

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `WEOF` | (wint_t)(-1) | `<wchar.h>` | 宽字符 EOF 标志，不可推回 |
| `MB_LEN_MAX` | 4 (典型) | `<limits.h>` | 多字节字符最大字节数 |
| `UNGET` | 8 | `stdio_impl.h` | 字符回退预留空间大小 |
| `F_EOF` | 16 | `stdio_impl.h` | 流 EOF 状态标志位 |
