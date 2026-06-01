# locale_impl.h 规约

> **来源文件**: `musl/src/internal/locale_impl.h`
> **复杂度层级**: Level 2 — 复杂逻辑（locale 管理 + 消息翻译 + 线程局部状态）
> **依赖图**:
> ```
> libc.h (提供 __locale_struct 定义)
>   -> pthread_impl.h (提供 __pthread_self() → locale 访问)
>     -> struct __locale_map — Locale 内存映射描述
>       -> __get_locale() — 按类别/名称查找 locale
>         -> __mo_lookup() — .mo 文件消息查找
>           -> __lctrans() / __lctrans_cur() / __lctrans_impl() — 翻译函数族
>             -> __loc_is_allocated() — locale_t 分配状态查询
>               -> __gettextdomain() — 线程局域文本域
>                 -> LCTRANS / LCTRANS_CUR / C_LOCALE / UTF8_LOCALE — 便捷宏
> ```

---

## 概述

`locale_impl.h` 定义 musl libc 中的 locale（区域设置）内部实现，包括 locale 数据的内存映射结构、消息翻译（.mo 文件）查找、以及线程局部 locale 状态的访问机制。该模块支撑 `setlocale()`、`localeconv()`、`gettext()` 等标准 C/POSIX locale 函数。

**不变量 (Invariants)**：
- **I1**: 全局 locale 数据（`__c_locale`、`__c_dot_utf8_locale`）在初始化后保持不变（不可变常量）。
- **I2**: 任何对线程 locale 的读取/写入（通过 `CURRENT_LOCALE`）必须在持有 `__locale_lock` 或不发生并发写入的安全上下文下进行。
- **I3**: `__locale_map.next` 构成单向链表，表示 locale 别名/继承链；遍历时无环。

---

## 常量定义

### `LOCALE_NAME_MAX`

```c
#define LOCALE_NAME_MAX 23
```

[Visibility]: Internal — musl 内部定义，POSIX/C 标准未规定

**意图**: locale 名称的最大字符数（不含结尾 `\0`）。POSIX 允许 locale 名为 `language_territory.codeset@modifier` 格式，23 字节足够容纳所有合法组合。

---

### `LOC_MAP_FAILED`

```c
#define LOC_MAP_FAILED ((const struct __locale_map *)-1)
```

[Visibility]: Internal — musl 内部哨兵值

**意图**: 用于标记 locale 映射查找失败但不应重试的哨兵指针。值为 `(const struct __locale_map *)-1`（即 `0xFFFFFFFFFFFFFFFF` 或 `0xFFFFFFFF`），表示"已尝试加载且失败，缓存此失败"。

---

## 结构体定义

### `struct __locale_map`

```c
struct __locale_map {
    const void *map;
    size_t map_size;
    char name[LOCALE_NAME_MAX+1];
    const struct __locale_map *next;
};
```

[Visibility]: Internal — musl 内部数据结构

**意图**: 描述一个被 `mmap` 加载到内存中的 locale 数据文件。该结构体同时充当链表节点（`next`），用于构建 locale 的别名链/fallback 链（如 `zh_CN.UTF-8` 不存在时退化为 `zh_CN`）。

**字段语义**：
| 字段 | 类型 | 语义 |
|------|------|------|
| `map` | `const void *` | `mmap` 映射到内存的 locale 数据文件基址（对 .mo 文件为消息目录，对 LC_CTYPE 等为字符映射表） |
| `map_size` | `size_t` | 映射区域大小（字节） |
| `name` | `char[24]` | locale 名称，以 `\0` 结尾（如 `"zh_CN.UTF-8"`），最大 23 字符 |
| `next` | `const struct __locale_map *` | 指向下一级 locale fallback 的指针，构成链表；链表尾为 `NULL` |

**关联**: `libc.h` 中的 `struct __locale_struct` 含 `const struct __locale_map *cat[6]`，对应 LC_CTYPE(0) ~ LC_MESSAGES(5)。

---

## 全局变量声明

### `__locale_lock`

```c
extern hidden volatile int __locale_lock[1];
```

[Visibility]: Internal — musl 内部全局锁

**意图**: 保护全局 locale 数据结构的自旋锁。任何修改 locale 映射表（`cat[]`）或创建新 `__locale_map` 的操作必须先获取此锁。

### `__c_dot_utf8`

```c
extern hidden const struct __locale_map __c_dot_utf8;
```

**意图**: C.UTF-8 locale 的 `__locale_map` 数据（静态编译在 musl 内部）。表示标准的 UTF-8 字符分类/转换表。

### `__c_locale` / `__c_dot_utf8_locale`

```c
extern hidden const struct __locale_struct __c_locale;
extern hidden const struct __locale_struct __c_dot_utf8_locale;
```

**意图**: 
- `__c_locale` — 标准 "C" locale，所有 `cat[]` 为 NULL 或指向内建 ASCII 表
- `__c_dot_utf8_locale` — "C.UTF-8" locale，cat[LC_CTYPE] 指向 UTF-8 表

---

## 函数声明

### `const struct __locale_map *__get_locale(int, const char *)`

```c
const struct __locale_map *__get_locale(int, const char *);
```

[Visibility]: Internal — musl 内部 locale 数据加载/查找

**意图 (Intent)**：
按类别编号（0~5）和名称查找或加载对应的 `__locale_map`。若名称对应的数据已加载，返回已有映射；否则尝试从文件系统加载对应 `.mo` 文件或 locale 数据。

**前置条件 (Preconditions)**：
- **P1**: `cat` 为 [0, 5] 范围内的有效 LC_* 类别编号（`LC_CTYPE=0` 到 `LC_MESSAGES=5`）。
- **P2**: `name` 非 NULL，为以 `\0` 结尾的有效 locale 名称。
- **P3**: 若需加载文件，调用者不应持有会竞争文件 I/O 的锁。

**后置条件 (Postconditions)**：
- **Case 1（成功）**：
  - **Q1**: 返回指向对应 `__locale_map` 的有效指针（`!= NULL && != LOC_MAP_FAILED`）。
  - **Q2**: 返回的映射包含已加载的 locale 数据和名称。
- **Case 2（失败）**：
  - **Q2a**: 返回 `LOC_MAP_FAILED`（若已尝试且失败，缓存失败状态），或
  - **Q2b**: 返回 `NULL`（若为 "C"/"POSIX" 等不需要映射的内建 locale）。

---

### `const char *__mo_lookup(const void *, size_t, const char *)`

```c
const char *__mo_lookup(const void *, size_t, const char *);
```

[Visibility]: Internal — musl 内部 .mo 文件查找

**意图 (Intent)**：
在已加载到内存的 GNU gettext .mo 格式消息文件中，二分查找给定原始字符串的翻译。实现 .mo 文件的哈希表 + 二分查找回退算法。

**前置条件 (Preconditions)**：
- **P1**: `map` 非 NULL，指向有效的 .mo 文件映射内存。
- **P2**: `size` 为 .mo 文件映射大小（不小于 .mo 文件头大小）。
- **P3**: `s` 非 NULL，为以 `\0` 结尾的原始文本（msgid）。

**后置条件 (Postconditions)**：
- **Case 1（找到翻译）**：返回指向翻译文本（msgstr）的指针。
- **Case 2（未找到翻译）**：返回 `s`（原文，即 fallback 到原始文本）。

**系统算法 (System Algorithm)**：
.mo 文件格式：文件头含哈希表偏移量和条目数。查找时先用哈希定位，若哈希冲突则在该哈希桶内二分查找。哈希函数为经典 PJW 哈希。

---

### `const char *__lctrans(const char *, const struct __locale_map *)`

```c
const char *__lctrans(const char *, const struct __locale_map *);
```

[Visibility]: Internal — musl 内部消息翻译（显式 locale）

**意图 (Intent)**：
将原始文本按照指定的 `__locale_map` 查找翻译。等价于 `__lctrans_impl(msg, loc_map)`。

**前置条件 (Preconditions)**：
- **P1**: `msg` 非 NULL，为以 `\0` 结尾的原始文本。
- **P2**: `loc_map` 可为 NULL（表示无翻译源，直接返回原文）。

**后置条件 (Postconditions)**：
- 若 `loc_map == NULL`：返回 `msg` 本身。
- 若 `loc_map != NULL`：在 `loc_map->map` 中调用 `__mo_lookup()`，返回翻译或原文。

---

### `const char *__lctrans_cur(const char *)`

```c
const char *__lctrans_cur(const char *);
```

[Visibility]: Internal — musl 内部消息翻译（当前线程 locale）

**意图 (Intent)**：
使用当前线程的 locale 设置（`CURRENT_LOCALE->cat[LC_MESSAGES]`）进行消息翻译。是 `gettext()` / `dgettext()` 的核心实现。

**前置条件 (Preconditions)**：
- **P1**: `msg` 非 NULL。
- **P2**: 当前线程的 `locale` 字段已初始化（至少为 `C_LOCALE`）。

**后置条件 (Postconditions)**：
- 返回经当前线程 locale 的 LC_MESSAGES 翻译后的文本（或原文）。

---

### `const char *__lctrans_impl(const char *, const struct __locale_map *)`

```c
const char *__lctrans_impl(const char *, const struct __locale_map *);
```

[Visibility]: Internal — musl 内部消息翻译底层实现

**意图 (Intent)**：
消息翻译的底层实现，包含对域名的预处理和 .mo 文件实际查找逻辑。被 `__lctrans()` 和 `__lctrans_cur()` 间接调用。

**前置条件 (Preconditions)**：
- **P1**: `msg` 非 NULL。
- **P2**: `loc_map` 可为 NULL（无翻译源时返回原文）。

**后置条件 (Postconditions)**：
- 同 `__lctrans()`，可能增加域名拼接逻辑。

---

### `int __loc_is_allocated(locale_t)`

```c
int __loc_is_allocated(locale_t);
```

[Visibility]: Internal — musl 内部 locale 生命周期管理

**意图 (Intent)**：
判断给定的 `locale_t` 对象是否为动态分配的（通过 `newlocale()` 创建），以决定其释放方式（free 还是 skip）。

**前置条件 (Preconditions)**：
- **P1**: `loc` 为有效的 `locale_t`（不能是已释放的悬空指针）。

**后置条件 (Postconditions)**：
- 返回 1：`loc` 为堆上动态分配（需 `free` 或 `__freelocale` 释放）
- 返回 0：`loc` 为静态内建 locale（`C_LOCALE` / `UTF8_LOCALE`，不可释放）

---

### `char *__gettextdomain(void)`

```c
char *__gettextdomain(void);
```

[Visibility]: Internal — musl 内部获取线程局域文本域

**意图 (Intent)**：
返回当前线程绑定的 gettext 域名（用于 `dgettext(NULL, msg)` 的域名获取）。

**前置条件 (Preconditions)**：
- 无。

**后置条件 (Postconditions)**：
- 返回指向域名缓冲区的 `char *`（可能为线程局部存储）。
- 返回内容可为空字符串（未设置域名时）。

---

## 宏定义

### `LCTRANS(msg, lc, loc)`

```c
#define LCTRANS(msg, lc, loc) __lctrans(msg, (loc)->cat[(lc)])
```

[Visibility]: Internal — musl 内部便捷宏

**意图**: 使用显式 locale 对象的指定类别进行翻译。`lc` 通常是 `LC_MESSAGES`。

### `LCTRANS_CUR(msg)`

```c
#define LCTRANS_CUR(msg) __lctrans_cur(msg)
```

[Visibility]: Internal — musl 内部便捷宏

**意图**: 使用当前线程 locale 进行翻译。是 `gettext(msg)` 的底层实现。

### `C_LOCALE`

```c
#define C_LOCALE ((locale_t)&__c_locale)
```

**意图**: 获取标准 "C" locale 的 `locale_t` 句柄。

### `UTF8_LOCALE`

```c
#define UTF8_LOCALE ((locale_t)&__c_dot_utf8_locale)
```

**意图**: 获取 "C.UTF-8" locale 的 `locale_t` 句柄。

### `CURRENT_LOCALE`

```c
#define CURRENT_LOCALE (__pthread_self()->locale)
```

**意图**: 通过线程控制块获取当前线程的 `locale_t`。依赖 `pthread_impl.h` 中的 `__pthread_self()`。

### `CURRENT_UTF8`

```c
#define CURRENT_UTF8 (!!__pthread_self()->locale->cat[LC_CTYPE])
```

**意图**: 判断当前线程 locale 是否为 UTF-8 编码。若 `cat[LC_CTYPE]` 指向有效的 UTF-8 映射表，则返回 1（真），否则返回 0。

### `MB_CUR_MAX`

```c
#undef MB_CUR_MAX
#define MB_CUR_MAX (CURRENT_UTF8 ? 4 : 1)
```

**意图**: 覆盖标准头文件中的 `MB_CUR_MAX` 定义为动态值。UTF-8 locale 下多字节字符最多 4 字节，非 UTF-8 locale 下为 1（单字节编码）。

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `struct __locale_struct` | `libc.h`（musl 内部） | 跨文件定义，含 `cat[6]` 数组成员 |
| `__pthread_self()` | `pthread_impl.h`（musl 内部） | 跨文件依赖，用于线程局部 locale 访问 |
| `__mo_lookup()` | `src/locale/__mo_lookup.c` | 跨文件实现 |
| `__lctrans_impl()` | `src/locale/` 相关文件 | 跨文件实现 |
| `<locale.h>`, `<stdlib.h>` | 标准 C 库 | 外部依赖 |

---

## 实现指南 (rusl/Rust)

- `struct __locale_map` → 使用 `#[repr(C)]` 结构体，`map` 字段为 `*const u8`，`next` 为 `Option<&'static __locale_map>` 或用 `*const __locale_map` 表示链表
- `__locale_lock` → `AtomicI32` 自旋锁
- `LOC_MAP_FAILED` → `usize::MAX` 哨兵值（或用 `Option<NonNull<__locale_map>>` 表示）
- locale 链表查找 → 使用 `unsafe` 遍历 `next` 指针链表
- `__mo_lookup()` → 手动解析 .mo 文件头部（`MAGIC = 0x950412de` 或 `0xde120495`），实现哈希查表 + 二分查找
- 线程局部 locale → 利用 Rust 的 `#[thread_local]` 或自定义 TLS 实现
- `MB_CUR_MAX` 的动态性需在运行时求值，不能是编译期常量