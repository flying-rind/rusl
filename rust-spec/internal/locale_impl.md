# locale_impl 规约 (Rust)

## 概述

`locale_impl` 模块定义 rusl libc 中的 locale(区域设置)内部实现，包括 locale 数据的内存映射结构、消息翻译(.mo 文件)查找、以及线程局部 locale 状态的访问机制。该模块支撑 `setlocale()`、`localeconv()`、`gettext()` 等标准 C/POSIX locale 函数。

**不变量 (Invariants)**:
- **I1**: 全局 locale 数据(`C_LOCALE`、`UTF8_LOCALE`)在初始化后保持不变(不可变常量)。
- **I2**: 任何对线程 locale 的读取/写入(通过 `CURRENT_LOCALE`)必须在持有 `locale_lock` 或不发生并发写入的安全上下文下进行。
- **I3**: `LocaleMap.next` 构成单向链表，表示 locale 别名/继承链; 遍历时无环。

## 依赖图

```
locale_impl 模块
├── libc 模块 (提供 LocaleStruct 定义)
├── pthread_impl 模块 (提供 __pthread_self() → locale 访问)
│
├── struct LocaleMap                  — Locale 内存映射描述
├── __get_locale(cat, name)           — 按类别/名称查找 locale
├── __mo_lookup(map, size, msg)       — .mo 文件消息查找
├── __lctrans(msg, loc_map)           — 翻译函数(显式 locale)
├── __lctrans_cur(msg)               — 翻译函数(当前线程 locale)
├── __lctrans_impl(msg, loc_map)      — 翻译函数底层实现
├── __loc_is_allocated(loc)           — locale_t 分配状态查询
├── __gettextdomain()                — 线程局域文本域
│
└── 便捷宏: LCTRANS / LCTRANS_CUR / C_LOCALE / UTF8_LOCALE / CURRENT_LOCALE / CURRENT_UTF8 / MB_CUR_MAX
```

---

```
/* Rely */
[RELY]
结构体依赖:
  struct LocaleStruct { cat: [*const LocaleMap; 6] }  // 依赖1: Locale 状态结构(定义于 libc 模块)
  struct pthread { locale: *const LocaleStruct, ... }  // 依赖2: 线程控制块(定义于 pthread_impl 模块)

函数依赖:
  fn __pthread_self() -> *mut pthread;                 // 依赖3: 获取当前线程控制块

内部依赖:
  fn __mo_lookup(map: *const u8, size: usize, msg: *const c_char) -> *const c_char;
                                                        // 依赖4: .mo 文件消息查找(跨文件实现)
  fn __lctrans_impl(msg: *const c_char, loc_map: *const LocaleMap) -> *const c_char;
                                                        // 依赖5: 消息翻译底层实现(跨文件实现)

系统依赖:
  mmap / open / close (系统调用)                        // 依赖6: locale 数据文件加载

[GUARANTEE]
内部接口:
  pub(crate) struct LocaleMap;                          // 本模块保证: locale 数据映射描述
  pub(crate) static mut locale_lock: i32;               // 本模块保证: 全局 locale 自旋锁
  pub(crate) static C_LOCALE: LocaleStruct;             // 本模块保证: C locale 常量
  pub(crate) static UTF8_LOCALE: LocaleStruct;          // 本模块保证: C.UTF-8 locale 常量
  pub(crate) fn __get_locale(cat: c_int, name: *const c_char) -> *const LocaleMap;
                                                        // 本模块保证: 按类别/名称查找
  pub(crate) fn __lctrans(msg: *const c_char, loc_map: *const LocaleMap) -> *const c_char;
                                                        // 本模块保证: 消息翻译(显式 locale)
  pub(crate) fn __lctrans_cur(msg: *const c_char) -> *const c_char;
                                                        // 本模块保证: 消息翻译(当前线程)
  pub(crate) fn __loc_is_allocated(loc: *const LocaleStruct) -> c_int;
                                                        // 本模块保证: locale_t 分配状态
  pub(crate) fn __gettextdomain() -> *mut c_char;       // 本模块保证: 线程文本域
```

---

## 常量定义

### LOCALE_NAME_MAX

```rust
// Rust — locale 名称最大长度
pub(crate) const LOCALE_NAME_MAX: usize = 23;
```

[Visibility]: Internal — rusl 内部定义

**意图**: locale 名称的最大字符数(不含结尾 `\0`)。POSIX 允许 locale 名为 `language_territory.codeset@modifier` 格式，23 字节足够容纳所有合法组合。

---

### LOC_MAP_FAILED (哨兵值)

```rust
// Rust — locale 映射查找失败哨兵
pub(crate) const LOC_MAP_FAILED: *const LocaleMap = usize::MAX as *const LocaleMap;
```

[Visibility]: Internal — rusl 内部哨兵值

**意图**: 用于标记 locale 映射查找失败但不应重试的哨兵指针。值为 `usize::MAX` 强转为 `*const LocaleMap`，表示"已尝试加载且失败，缓存此失败以避免重复文件 I/O"。

**Rust 设计说明**: C 中使用 `(const struct __locale_map *)-1` 作为哨兵。Rust 中 `usize::MAX` 等价于 `-1isize as usize`，强转为指针后语义一致。在所有现代平台上该地址不可达(通常位于内核空间)，因此可安全用作哨兵。

---

## 结构体定义

### LocaleMap

```rust
// Rust — locale 数据内存映射描述 (同时充当链表节点)
#[repr(C)]
pub(crate) struct LocaleMap {
    pub map: *const u8,                    // mmap 映射到的 locale 数据文件基址
    pub map_size: usize,                   // 映射区域大小(字节)
    pub name: [u8; LOCALE_NAME_MAX + 1],   // locale 名称, 以 '\0' 结尾, 最大 23 字符
    pub next: *const LocaleMap,            // 下一级 locale fallback 指针, 链表尾为 null
}
```

[Visibility]: Internal — rusl 内部数据结构

**意图**: 描述一个被 `mmap` 加载到内存中的 locale 数据文件。该结构体同时充当链表节点(`next`)，用于构建 locale 的别名链/fallback 链(如 `zh_CN.UTF-8` 不存在时退化为 `zh_CN`)。

**字段语义**:
| 字段 | 类型 | 语义 |
|------|------|------|
| `map` | `*const u8` | `mmap` 映射到内存的 locale 数据文件基址(对 .mo 文件为消息目录，对 LC_CTYPE 等为字符映射表) |
| `map_size` | `usize` | 映射区域大小(字节) |
| `name` | `[u8; 24]` | locale 名称，以 `\0` 结尾(如 `"zh_CN.UTF-8"`)，最大 23 字符 |
| `next` | `*const LocaleMap` | 指向下一级 locale fallback 的指针，构成链表; 链表尾为 `null` |

**关联**: `libc` 模块中的 `LocaleStruct` 含 `cat: [*const LocaleMap; 6]`，对应 LC_CTYPE(0) ~ LC_MESSAGES(5)。

**不变量**: 若 `map` 非 null，则 `map` 指向的内存区域已通过 `mmap` 映射，大小为 `map_size`，内容为对应 locale 类别数据或 .mo 消息文件。

---

## 全局变量声明

### locale_lock

```rust
// Rust — 保护全局 locale 数据结构访问的自旋锁
pub(crate) static mut locale_lock: i32 = 0;
// 或使用 AtomicI32::new(0) 并实现自旋锁语义
```

[Visibility]: Internal — rusl 内部全局锁

**意图**: 保护全局 locale 数据结构的自旋锁。任何修改 locale 映射表(`cat[]`)或创建新 `LocaleMap` 的操作必须先获取此锁。

**Rust 设计说明**: C 中使用 `volatile int[1]` 数组实现自旋锁。Rust 中可使用 `static mut AtomicI32` 或自定义 `SpinLock` 类型。由于涉及 `mmap` 和文件 I/O，持有锁的时间可能较长，因此不适合使用 `Mutex`(非 no_std)。推荐使用 aarch64/x86 的 `compare_exchange` 原子指令实现自旋锁。

---

### 内建 locale 常量

```rust
// Rust — 内建 locale 数据常量
pub(crate) static C_LOCALE: LocaleStruct = LocaleStruct { cat: [null(); 6] };
pub(crate) static UTF8_LOCALE: LocaleStruct = LocaleStruct { cat: [
    &__c_dot_utf8 as *const LocaleMap, // LC_CTYPE: 指向 UTF-8 字符分类表
    null(), null(), null(), null(), null(),
]};
pub(crate) static __c_dot_utf8: LocaleMap = LocaleMap {
    map: /* 静态编译的 UTF-8 字符映射表地址 */,
    map_size: /* 表大小 */,
    name: *b"C.UTF-8\0",
    next: null(),
};
```

[Visibility]: Internal — rusl 内部常量

**意图**:
- `C_LOCALE` — 标准 "C" locale，所有 `cat[]` 为 null(表示使用内建 ASCII 语义)
- `UTF8_LOCALE` — "C.UTF-8" locale，`cat[LC_CTYPE]` 指向 UTF-8 字符分类表
- `__c_dot_utf8` — C.UTF-8 locale 的 `LocaleMap` 数据(静态编译在 rusl 内部)

---

## 函数声明

### __get_locale

```rust
// Rust — 按类别和名称查找/加载 locale 数据
fn __get_locale(cat: c_int, name: *const c_char) -> *const LocaleMap;
```

[Visibility]: Internal — rusl 内部 locale 数据加载/查找

**意图 (Intent)**:
按类别编号(0~5)和名称查找或加载对应的 `LocaleMap`。若名称对应的数据已加载，返回已有映射; 否则尝试从文件系统加载对应 `.mo` 文件或 locale 数据文件。

**前置条件 (Preconditions)**:
- **P1**: `cat` 为 [0, 5] 范围内的有效 LC_* 类别编号(`LC_CTYPE=0` 到 `LC_MESSAGES=5`)。
- **P2**: `name` 非 null，为以 `\0` 结尾的有效 locale 名称。
- **P3**: 若需加载文件，调用者不应持有会竞争文件 I/O 的锁。

**后置条件 (Postconditions)**:
- **Case 1 (成功)**: 返回指向对应 `LocaleMap` 的有效指针(`!= null && != LOC_MAP_FAILED`)。
- **Case 2 (失败且已缓存)**: 返回 `LOC_MAP_FAILED`(已尝试加载且失败，缓存失败状态)。
- **Case 3 (内建 locale)**: 返回 `null`("C"/"POSIX" 等不需要映射的内建 locale)。

---

### __mo_lookup

```rust
// Rust — 在已加载的 .mo 文件中查找消息翻译
fn __mo_lookup(map: *const u8, size: usize, msg: *const c_char) -> *const c_char;
```

[Visibility]: Internal — rusl 内部 .mo 文件查找

**意图 (Intent)**:
在已加载到内存的 GNU gettext .mo 格式消息文件中，二分查找给定原始字符串的翻译。实现 .mo 文件的哈希表 + 二分查找回退算法。

**前置条件 (Preconditions)**:
- **P1**: `map` 非 null，指向有效的 .mo 文件映射内存。
- **P2**: `size` 为 .mo 文件映射大小(不小于 .mo 文件头大小)。
- **P3**: `msg` 非 null，为以 `\0` 结尾的原始文本(msgid)。

**后置条件 (Postconditions)**:
- **Case 1 (找到翻译)**: 返回指向翻译文本(msgstr)的指针。
- **Case 2 (未找到翻译)**: 返回 `msg` 本身(原文，即 fallback 到原始文本)。

**System Algorithm**:
.mo 文件格式: 文件头含哈希表偏移量和条目数。查找时先用哈希定位，若哈希冲突则在该哈希桶内二分查找。哈希函数为经典 PJW 哈希。

**Rust 设计说明**: C 中使用 `(const void *, size_t, const char *)`。Rust 中 `map` 使用 `*const u8` 明确表示字节级内存访问语义。

---

### __lctrans

```rust
// Rust — 使用显式 locale 映射进行消息翻译
fn __lctrans(msg: *const c_char, loc_map: *const LocaleMap) -> *const c_char;
```

[Visibility]: Internal — rusl 内部消息翻译(显式 locale)

**意图 (Intent)**:
将原始文本按照指定的 `LocaleMap` 查找翻译。等价于调用 `__lctrans_impl(msg, loc_map)`。

**前置条件 (Preconditions)**:
- **P1**: `msg` 非 null，为以 `\0` 结尾的原始文本。
- **P2**: `loc_map` 可为 null(表示无翻译源，直接返回原文)。

**后置条件 (Postconditions)**:
- 若 `loc_map == null`: 返回 `msg` 本身。
- 若 `loc_map != null`: 在 `loc_map.map` 中调用 `__mo_lookup()`，返回翻译或原文。

---

### __lctrans_cur

```rust
// Rust — 使用当前线程 locale 进行消息翻译
fn __lctrans_cur(msg: *const c_char) -> *const c_char;
```

[Visibility]: Internal — rusl 内部消息翻译(当前线程 locale)

**意图 (Intent)**:
使用当前线程的 locale 设置(`CURRENT_LOCALE.cat[LC_MESSAGES]`)进行消息翻译。是 `gettext()` / `dgettext()` 的核心实现。

**前置条件 (Preconditions)**:
- **P1**: `msg` 非 null。
- **P2**: 当前线程的 `locale` 字段已初始化(至少为 `C_LOCALE`)。

**后置条件 (Postconditions)**:
- 返回经当前线程 locale 的 LC_MESSAGES 翻译后的文本(或原文)。

---

### __lctrans_impl

```rust
// Rust — 消息翻译底层实现
fn __lctrans_impl(msg: *const c_char, loc_map: *const LocaleMap) -> *const c_char;
```

[Visibility]: Internal — rusl 内部消息翻译底层实现

**意图 (Intent)**:
消息翻译的底层实现，包含对域名的预处理和 .mo 文件实际查找逻辑。被 `__lctrans()` 和 `__lctrans_cur()` 间接调用。

**前置条件 (Preconditions)**:
- **P1**: `msg` 非 null。
- **P2**: `loc_map` 可为 null(无翻译源时返回原文)。

**后置条件 (Postconditions)**:
- 同 `__lctrans()`，可能增加域名拼接逻辑(将 `__gettextdomain()` 的域名与 msgid 组合查找)。

---

### __loc_is_allocated

```rust
// Rust — 判断 locale_t 是否堆上分配
fn __loc_is_allocated(loc: *const LocaleStruct) -> c_int;
```

[Visibility]: Internal — rusl 内部 locale 生命周期管理

**意图 (Intent)**:
判断给定的 `locale_t` 对象是否为动态分配的(通过 `newlocale()` 创建)，以决定其释放方式(free 还是 skip)。

**前置条件 (Preconditions)**:
- **P1**: `loc` 为有效的 `locale_t`(不能是已释放的悬空指针)。

**后置条件 (Postconditions)**:
- 返回 1: `loc` 为堆上动态分配(需 `free` 或 `__freelocale` 释放)
- 返回 0: `loc` 为静态内建 locale(`C_LOCALE` / `UTF8_LOCALE`，不可释放)

---

### __gettextdomain

```rust
// Rust — 获取当前线程绑定的 gettext 域名
fn __gettextdomain() -> *mut c_char;
```

[Visibility]: Internal — rusl 内部获取线程局域文本域

**意图 (Intent)**:
返回当前线程绑定的 gettext 域名(用于 `dgettext(NULL, msg)` 的域名获取)。

**前置条件 (Preconditions)**:
- 无。

**后置条件 (Postconditions)**:
- 返回指向域名缓冲区的 `*mut c_char`(可能为线程局部存储)。
- 返回内容可为空字符串(未设置域名时)。

---

## 宏/内联函数

### LCTRANS

```rust
// Rust — 使用显式 locale 对象的指定类别进行翻译
#[inline]
pub(crate) fn lctrans(msg: *const c_char, lc: c_int, loc: &LocaleStruct) -> *const c_char {
    // 注意: loc.cat[lc] 可为 null
    __lctrans(msg, loc.cat[lc as usize])
}
```

[Visibility]: Internal — rusl 内部便捷函数

**意图**: 使用显式 locale 对象的指定类别进行翻译。`lc` 通常是 `LC_MESSAGES`。

**Rust 设计说明**: C 中使用宏 `#define LCTRANS(msg, lc, loc) __lctrans(msg, (loc)->cat[(lc)])`。Rust 中改为内联函数，享受类型检查且调试更友好。`lc as usize` 的索引需调用者保证在 [0, 5] 范围内(debug 模式可用 `debug_assert!` 检查)。

---

### LCTRANS_CUR

```rust
// Rust — 使用当前线程 locale 进行翻译
#[inline]
pub(crate) fn lctrans_cur(msg: *const c_char) -> *const c_char {
    __lctrans_cur(msg)
}
```

[Visibility]: Internal — rusl 内部便捷函数

**意图**: 使用当前线程 locale 进行翻译。是 `gettext(msg)` 的底层实现。

---

### C_LOCALE / UTF8_LOCALE 句柄

```rust
// Rust — 获取内建 locale 指针
pub(crate) fn c_locale() -> *const LocaleStruct { &C_LOCALE as *const LocaleStruct }
pub(crate) fn utf8_locale() -> *const LocaleStruct { &UTF8_LOCALE as *const LocaleStruct }
```

**意图**: 获取标准 "C" locale 和 "C.UTF-8" locale 的 `locale_t` 句柄(裸指针)。

**Rust 设计说明**: C 中使用宏 `#define C_LOCALE ((locale_t)&__c_locale)`。Rust 中改为内联函数，避免宏的展开问题，且能参与类型检查。在 release 编译中 LTO 会将其彻底内联为常量地址。

---

### CURRENT_LOCALE

```rust
// Rust — 获取当前线程的 locale
#[inline]
pub(crate) unsafe fn current_locale() -> *const LocaleStruct {
    // 通过线程控制块获取当前线程的 locale 设置
    (*__pthread_self()).locale
}
```

**意图**: 通过线程控制块获取当前线程的 `locale_t`。依赖 `pthread_impl` 模块中的 `__pthread_self()`。

**Rust 设计说明**: 使用 `unsafe` 函数封装裸指针解引用，集中管理 unsafety。

---

### CURRENT_UTF8

```rust
// Rust — 判断当前线程 locale 是否为 UTF-8 编码
#[inline]
pub(crate) unsafe fn current_utf8() -> bool {
    !(*__pthread_self()).locale.cat[LC_CTYPE as usize].is_null()
}
```

**意图**: 判断当前线程 locale 是否为 UTF-8 编码。若 `cat[LC_CTYPE]` 指向有效的 UTF-8 映射表，则返回 `true`，否则返回 `false`。

---

### MB_CUR_MAX

```rust
// Rust — 当前 locale 下多字节字符最大字节数
// 注意: 这是运行时值，不能是编译期常量
pub(crate) fn mb_cur_max() -> usize {
    if unsafe { current_utf8() } { 4 } else { 1 }
}
```

**意图**: 覆盖标准头文件中的 `MB_CUR_MAX` 定义为动态值。UTF-8 locale 下多字节字符最多 4 字节，非 UTF-8 locale 下为 1(单字节编码)。

**Rust 设计说明**: C 中的 `#define MB_CUR_MAX (CURRENT_UTF8 ? 4 : 1)` 依赖预处理器和宏展开。Rust 中必须使用函数调用在运行时求值。公共 API 中暴露为 `extern "C"` 函数以保持 ABI 兼容(若 C 标准要求 `MB_CUR_MAX` 为宏，则需要特殊处理)。

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `LocaleStruct` | `libc` 模块(rusl 内部) | 跨模块定义，含 `cat: [*const LocaleMap; 6]` |
| `__pthread_self()` | `pthread_impl` 模块(rusl 内部) | 跨模块依赖，用于线程局部 locale 访问 |
| `__mo_lookup()` | `src/locale/__mo_lookup.rs` | 跨文件实现 |
| `__lctrans_impl()` | `src/locale/` 相关文件 | 跨文件实现 |

---

## 实现指南 (rusl/Rust)

- `LocaleMap` 使用 `#[repr(C)]` 结构体，`map` 字段为 `*const u8`，`next` 为 `*const LocaleMap`
- `locale_lock` 使用 `AtomicI32` 自旋锁，配合 `core::sync::atomic::Ordering::Acquire/Release` 内存顺序
- `LOC_MAP_FAILED` 使用 `usize::MAX as *const LocaleMap` 哨兵值
- locale 链表查找: 使用 `unsafe` 遍历 `next` 指针链表，注意检查环路
- `__mo_lookup()`: 手动解析 .mo 文件头部(`MAGIC = 0x950412de` 或 `0xde120495`)，实现 PJW 哈希查表 + 二分查找
- 线程局部 locale: 利用 Rust 的 `#[thread_local]` 属性或通过 `__pthread_self()` 访问线程控制块
- `MB_CUR_MAX`: 由于必须在运行时求值，考虑在公共头文件中导出为函数而非宏(或通过 `extern "C" fn __mb_cur_max() -> usize` 桥接)
- 所有 `extern "C"` 导出的函数必须使用与 C spec 一致的调用约定和参数类型