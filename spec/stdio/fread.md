# fread.c 规约

> musl libc 标准 IO 二进制块读取实现。从 FILE 流中读取指定数量的元素到用户缓冲区，提供 `fread`（加锁）和 `fread_unlocked`（免锁）两个接口。

---

## 依赖图

```
fread (Public)
  ├── FLOCK / FUNLOCK (锁宏, 定义于 stdio_impl.h)
  │     ├── __lockfile (see __lockfile.c spec)
  │     └── __unlockfile (see __unlockfile.c spec)
  ├── memcpy (from <string.h>)
  ├── __toread (see __toread.c spec)
  └── f->read (FILE 函数指针, typically __stdio_read)
```

---

## 内部宏定义

### MIN

```c
#define MIN(a,b) ((a)<(b) ? (a) : (b))
```

[Visibility]: Internal — 文件内部宏，不对外导出

求两值较小者。

---

## 函数规约

### 1. fread

```c
size_t fread(void *restrict destv, size_t size, size_t nmemb, FILE *restrict f);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用

#### Intent

从 FILE 流 `f` 中读取 `nmemb` 个大小为 `size` 字节的元素到 `destv` 缓冲区。采用两阶段策略：先耗尽 FILE 内部读缓冲区的已有数据（`rpos` 到 `rend` 之间），再通过底层 `read` 函数直接读取剩余数据，减少内存拷贝。

#### 前置条件

- `destv`: 非空指针（除非 `size` 或 `nmemb` 为 0），指向至少 `size * nmemb` 字节的有效可写内存
- `size`: 每个元素的大小（字节），可为 0
- `nmemb`: 要读取的元素数量，可为 0
- `f`: 非空 FILE 指针，指向已打开的流（读模式或可读模式）

#### 后置条件

- **Case 1 完全成功读取全部元素**
  - 返回 `nmemb`（要求读取的元素总数）
  - `destv` 中包含完整数据

- **Case 2 读取部分元素后遇到 EOF 或出错**
  - 返回 `< nmemb`（实际完整读取的元素数，即 `(已读取字节数) / size`）
  - `destv` 中前 `返回值 * size` 字节有效
  - FILE 流相关标志位被设置（`F_EOF` 或 `F_ERR`）

- **Case 3 `size` 为 0**
  - 返回 0，不执行任何读取操作

#### 系统算法

```
fread(destv, size, nmemb, f):
  len = size * nmemb                    // 总字节数
  l = len                                // 剩余待读字节数
  if size == 0: nmemb = 0

  FLOCK(f)                               // 获取 FILE 锁

  f->mode |= f->mode-1                   // 设置读模式 (mode 最低位变 1)

  // 阶段 1: 耗尽 FILE 内部读缓冲区
  if f->rpos != f->rend:
    k = MIN(f->rend - f->rpos, l)        // 可拷贝的字节数
    memcpy(dest, f->rpos, k)
    f->rpos += k
    dest += k
    l -= k

  // 阶段 2: 直接读取剩余数据
  while l > 0:
    k = __toread(f) ? 0 : f->read(f, dest, l)
    if k == 0:                           // EOF 或错误
      FUNLOCK(f)
      return (len - l) / size            // 返回已完整读取的元素数
    dest += k
    l -= k

  FUNLOCK(f)
  return nmemb
```

**模式设置技巧**: `f->mode |= f->mode-1` 是 musl 特有的位操作。FILE 的 `mode` 字段最低位: 0 表示未读模式，1 表示写模式。`mode-1` 若 mode 为 0 则得 -1（全 1），`mode |= -1` 将其设为全 1；若 mode 为 1 则 `mode |= 0` 保持不变。最终效果是确保 mode 包含了读模式标志。

#### 不变量

- `f->lock` 在整个执行期间被当前线程持有（FLOCK/FUNLOCK 配对）
- `dest + (len - l)` 始终等于已写入 dest 的数据位置
- `l` 始终等于剩余待读字节数

#### 依赖

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁宏
- `memcpy` — 内存拷贝（`<string.h>`）
- `__toread(FILE *)` — 确保 FILE 处于读模式（internal）
- `f->read` — FILE 底层读函数指针（通常指向 `__stdio_read`）

---

### 2. fread_unlocked (weak_alias)

```c
// weak_alias(fread, fread_unlocked);
size_t fread_unlocked(void *restrict destv, size_t size, size_t nmemb, FILE *restrict f);
```

[Visibility]: User — 标准库函数，语义上的免锁版本（与 musl 中 `fread` 共享同一实现，因为 musl 的 fread 本身就是线程安全的加锁版本；此弱别名存在是为了 POSIX 兼容）

#### Intent

POSIX 标准定义的免锁 `fread`。在 musl 实现中与 `fread` 是完全相同的实现（musl 的 `fread` 总是以线程安全方式实现，宏 FLOCK/FUNLOCK 确保锁定行为）。

前置/后置条件及行为：完全等同于 `fread`。
