# setenv.c 规约

## 依赖图

```
setenv
  ├── __strchrnul(const char *, int)
  │     → src/string/strchrnul.c (跨模块内部函数)
  ├── getenv(const char *)
  │     → src/env/getenv.c (跨模块，POSIX 公开接口)
  ├── strlen(const char *)
  │     → libc (外部标准库)
  ├── malloc(size_t)
  │     → libc (外部标准库)
  ├── memcpy(void *, const void *, size_t)
  │     → libc (外部标准库)
  ├── __putenv(char *, size_t, char *)
  │     → src/env/putenv.c (跨模块内部函数)
  │       ├── strlen, strncmp, memcpy, malloc, realloc, free (外部)
  │       ├── __strchrnul (跨模块内部)
  │       ├── __environ (跨模块全局变量)
  │       └── __env_rm_add → 本文件 (强定义，putenv.c 中为 weak 备选)
  └── errno / EINVAL
        → libc (外部标准库)

__env_rm_add
  ├── free(void *)
  │     → libc (外部标准库)
  ├── realloc(void *, size_t)
  │     → libc (外部标准库)
  └── [static] env_alloced, env_alloced_n
        → 本文件内部静态变量，用于追踪堆分配的 environment 字符串
```

> **关键架构说明**: `__env_rm_add` 在 `setenv.c` 中为强符号定义（strong definition），提供完整的内存追踪实现。`putenv.c`、`unsetenv.c`、`clearenv.c` 中通过 `static void dummy(char *, char *) {}; weak_alias(dummy, __env_rm_add);` 提供弱符号备选（no-op 桩）。当 `setenv.c` 被链接到最终二进制时（因 `setenv` 被使用），此强定义覆盖弱备选，启用环境字符串的自动垃圾回收。这保证了未使用 `setenv` 的程序不承担额外的内存管理开销。

---

## __env_rm_add (内部函数)

```c
void __env_rm_add(char *old, char *new);
```

**[Visibility]: Internal (不导出)** — musl 内部环境变量内存管理函数。声明于 `src/include/stdlib.h`（`hidden` 可见性），用户代码不可直接调用。POSIX 和 C 标准均未定义此函数。

### 意图 (Intent)

管理进程环境变量字符串的生命周期。该函数维护一个动态数组 `env_alloced`，追踪所有由 musl 堆分配的环境变量字符串（"NAME=VALUE" 格式）。当环境变量被替换或删除时，负责释放旧字符串；当新字符串被创建时，负责将其加入追踪列表，确保进程退出时所有分配的环境字符串可被回收。

### 涉及的静态状态

```c
static char **env_alloced;      // 动态数组，元素为指向已分配环境变量字符串的指针
static size_t env_alloced_n;    // env_alloced 数组中的条目数
```

**不变量 (Invariant)**:
1. `env_alloced` 中非 NULL 条目均为指向堆上 "NAME=VALUE" 格式字符串的有效指针。
2. `env_alloced` 中可能包含 NULL 条目（表示已被释放但槽位待复用的位置）。
3. `env_alloced_n` 等于 `env_alloced` 数组的逻辑长度（即分配的元素数量）。
4. `env_alloced` 仅由 `__env_rm_add` 修改，其他模块仅读取（通过 `getenv`）或间接修改（通过 `__putenv` 传递参数）。
5. 同一条 env 字符串指针在 `env_alloced` 中至多出现一次（无重复追踪）。

### 前置条件 (Precondition)

- 无特殊前提条件。`old` 和 `new` 可为任意指针（包括 `NULL`）。
- 调用者（`__putenv`、`unsetenv`、`clearenv`）保证：
  - 当 `old` 非 NULL 时，它指向一个之前已通过 `__env_rm_add` 追踪的环境变量字符串，且调用者正在从 `environ` 中移除或替换该条目。
  - 当 `new` 非 NULL 时，它指向一个堆上新分配的 "NAME=VALUE" 格式字符串，需要被追踪以便未来释放。

### 后置条件 (Postcondition)

分四种典型调用模式描述：

**Case 1 — 替换环境变量（`old != NULL`，`new != NULL`）**:
- 在 `env_alloced` 中查找匹配 `old` 的条目。
- 若找到：用 `new` 替换该条目，调用 `free(old)` 释放旧字符串。
- 若 `old` 不在 `env_alloced` 中（如源自父进程的原始 environ）：将 `new` 追加到 `env_alloced`（复用 NULL 槽或 realloc 扩容）。
- 无返回值，操作保证成功（`realloc` 失败时静默丢弃 `new` 的追踪，但旧字符串正常释放）。

**Case 2 — 删除环境变量（`old != NULL`，`new == NULL`）**:
- 在 `env_alloced` 中查找匹配 `old` 的条目。
- 若找到：将该条目标记为 NULL（释放槽位），调用 `free(old)` 释放旧字符串。
- 若未找到：无操作（`old` 来自父进程 environ，由父进程管理生命周期）。
- 无返回值。

**Case 3 — 添加新环境变量（`old == NULL`，`new != NULL`）**:
- 首先尝试在 `env_alloced` 中寻找已有的 NULL 槽位复用。
- 若找到 NULL 槽位：将 `new` 填入该槽位。
- 若未找到：调用 `realloc(env_alloced, sizeof(char*) * (env_alloced_n + 1))` 扩容数组，将 `new` 追加到末尾，`env_alloced_n++`。
- 若 `realloc` 失败：静默返回，`new` 不被追踪（内存泄漏，但为有限泄漏且进程通常即将终止）。

**Case 4 — NOP（`old == NULL`，`new == NULL`）**:
- 无任何操作，立即返回。（此情形在线性扫描中自然满足：循环无匹配后 `!new` 成立，直接返回）

### 系统算法 (System Algorithm)

使用单次线性扫描 + 复用空闲槽的策略：

```
for i = 0 to env_alloced_n - 1:
    if env_alloced[i] == old:        // 找到要替换的条目
        env_alloced[i] = new
        free(old)
        return
    else if env_alloced[i] == NULL and new != NULL:  // 找到空闲槽
        env_alloced[i] = new          // 填入新值
        new = 0                       // 标记已放置
// 循环结束
if new == NULL:  return                // 已放置或无需操作
// 否则：需要扩容
t = realloc(env_alloced, sizeof(char*) * (env_alloced_n + 1))
if t == NULL:  return                  // 分配失败，静默放弃追踪
env_alloced = t
env_alloced[env_alloced_n++] = new
```

**关键设计决策**:
1. **单次扫描、双条件检查**: 在同一循环中同时检查"替换"和"放置"条件。这避免了两次独立的扫描，但引入一个微妙语义：当 `new` 在 NULL 槽被放置（`new = 0`）后，若后续迭代匹配到 `old`，`env_alloced[i] = 0` 会将 old 的槽位清零（而非常规替换），此行为对调用者透明，无副作用。
2. **NULL 槽复用**: 未被 `realloc` 回收的槽位保持为 NULL，等待后续添加操作复用。这避免了频繁的 realloc 调用。
3. **静默 realloc 失败**: 不在 realloc 失败时返回错误，因为此时系统已接近 OOM，且丢失对单个环境字符串的追踪不影响程序继续运行（字符串本身已在 `environ` 中可用）。

### 线程安全

本函数**不是**线程安全的。它操作静态变量 `env_alloced` 和 `env_alloced_n` 而无任何同步机制。POSIX 标准明确规定 `setenv`、`unsetenv`、`putenv` 等函数不是线程安全的，故调用者负责外部同步，本函数无需内部加锁。

---

## setenv (对外导出)

```c
int setenv(const char *var, const char *value, int overwrite);
```

**[Visibility]: Public** — POSIX.1-2001 标准函数，声明于 `<stdlib.h>`。用户程序可直接调用。

### 意图 (Intent)

向进程环境变量列表中添加或更新一个环境变量。该函数构造 "NAME=VALUE" 格式的字符串并将其插入到环境变量数组中。与 `putenv` 不同，`setenv` 会自行分配并复制字符串，而非要求调用者管理内存。

### 前置条件 (Precondition)

1. `var` 必须为指向以 NUL 结尾的非空 C 字符串的有效指针。
2. `var` 的内容必须满足以下所有条件：
   - 非空字符串（长度 > 0）。
   - 不包含 `=` 字符。
3. `value` 必须为指向以 NUL 结尾的 C 字符串的有效指针（允许空字符串 `""`）。
4. `overwrite` 取值为 `0` 或非 0 整数。
5. 调用者负责确保线程安全（POSIX 未规定 `setenv` 为线程安全函数）。

### 后置条件 (Postcondition)

**Case 1 — 参数校验失败**:
- 条件: `var == NULL`，或 `var` 长度为 0（空字符串），或 `var` 中包含 `=` 字符。
- 行为: 设置 `errno = EINVAL`，返回 `-1`。
- 环境变量列表不发生任何变化。

**Case 2 — 变量已存在且 `overwrite == 0`**:
- 条件: 校验通过，`getenv(var)` 返回非 NULL，且 `overwrite == 0`。
- 行为: 返回 `0`，环境变量列表不发生任何变化。
- 不分配新内存。

**Case 3 — 内存分配失败**:
- 条件: 校验通过，且（变量不存在 或 `overwrite != 0`），但 `malloc(l1 + l2 + 2)` 返回 NULL。
- 行为: 返回 `-1`。`errno` 的值取决于 `malloc` 的实现（通常为 `ENOMEM`）。
- 环境变量列表不发生任何变化。

**Case 4 — 成功添加/更新**:
- 条件: 校验通过，且（变量不存在 或 `overwrite != 0`），且内存分配成功。
- 行为:
  1. 分配大小为 `strlen(var) + strlen(value) + 2` 字节的堆内存。
  2. 在此内存中构造字符串 `"var=value"`（格式: `var` + `=` + `value` + `\0`）。
  3. 调用 `__putenv(s, l1, s)` 将构造的字符串插入环境变量数组:
     - 若 `var` 已存在对应条目：替换旧条目，通过 `__env_rm_add` 释放旧字符串。
     - 若 `var` 不存在：在 `__environ` 数组中追加新条目，通过 `__env_rm_add` 追踪新分配的字符串。
  4. 返回 `0`（`__putenv` 的返回值，成功为 0）。
- 副作用:
  - `__environ` 数组可能被重新分配（扩容）。
  - `__environ` 中特定条目的指针可能被修改。
  - `env_alloced`（`__env_rm_add` 的静态追踪数组）可能被更新。

### 系统算法 (System Algorithm)

```
1. 校验 var 参数:
   - 若 var == NULL: goto invalid
   - 调用 __strchrnul(var, '=') 找到 '=' 或末尾 NUL
   - 计算 l1 = (找到位置) - var (即 var 的长度)
   - 若 l1 == 0 (空字符串): goto invalid
   - 若 var[l1] != '\0' (找到了 '=', 即 var 中包含 '='): goto invalid

2. 检查 overwrite 策略:
   - 若 overwrite == 0 且 getenv(var) != NULL: return 0 (已存在且不覆盖)

3. 构造新字符串:
   - l2 = strlen(value)
   - s = malloc(l1 + l2 + 2)    // +2: '=' + '\0'
   - 若 s == NULL: return -1
   - memcpy(s, var, l1)         // 复制变量名
   - s[l1] = '='                // 设置分隔符
   - memcpy(s + l1 + 1, value, l2 + 1)  // 复制值及末尾 NUL

4. 插入环境:
   - return __putenv(s, l1, s)   // __putenv 负责查找替换或追加，失败时释放 s

invalid:
   - errno = EINVAL
   - return -1
```

### 参数语义说明

- **`var`**: 环境变量名。不能为空，不能含 `=`。因为 `=` 是环境变量"名称=值"格式的分隔符。
- **`value`**: 环境变量值。可以为空字符串 `""`，此时环境变量被设置为空值（如 `"PATH="`）。
- **`overwrite`**: 覆盖标志。
  - `0`: 若变量已存在，不修改其值，返回成功。这允许"默认值"语义。
  - 非 0: 无论变量是否存在，始终更新其值。

### POSIX 标准兼容性

本实现符合 POSIX.1-2001 规范：

| POSIX 要求 | musl 实现 |
|---|---|
| `var` 含 `=` 时返回 -1，设置 `errno = EINVAL` | 符合 |
| `var` 为空字符串的行为 | POSIX 未规定，musl 返回 -1 + EINVAL |
| 成功返回 0 | 符合 |
| 失败返回 -1 | 符合 |
| 分配并复制字符串 | 符合（`malloc` + `memcpy`） |

### 与 putenv 的对比

| 特性 | setenv | putenv |
|---|---|---|
| 字符串所有权 | musl 分配并拥有（加入 `__env_rm_add` 追踪） | 调用者拥有（`putenv` 仅存储指针，不复制） |
| 自动释放 | 是（通过 `__env_rm_add` 追踪） | 否（调用者负责生命周期） |
| 接口复杂度 | 更高（需要分别传 name 和 value） | 更简单（传 "NAME=VALUE" 字符串） |
| 内存安全性 | 更安全（内部管理） | 危险（调用者修改字符串会影响环境） |

### 跨模块依赖说明

1. **`__strchrnul`** (`src/string/strchrnul.c`): 内部字符串查找函数。返回指向 `s` 中字符 `c` 首次出现位置的指针，若未找到则返回指向末尾 NUL 的指针。用于高效计算 `var` 长度并同时检测是否含 `=`。

2. **`__putenv`** (`src/env/putenv.c`): 内部环境变量插入核心函数。负责在 `__environ` 数组中查找/替换/追加条目，管理 `__environ` 的重新分配，以及调用 `__env_rm_add` 追踪堆分配字符串的生命周期。详见 `src/env/spec/putenv.md`（若存在）。

3. **`getenv`** (`src/env/getenv.c`): POSIX 标准函数。在 `__environ` 中按名称查找环境变量并返回值字符串指针。用于 `overwrite == 0` 时检查变量是否已存在。

4. **`__env_rm_add`**: 本文件定义的强符号。由 `__putenv` 通过函数指针（链接时解析为强符号）调用，负责追踪和释放堆分配的环境字符串。

5. **`__environ`** (`src/env/__environ.c`): 全局变量，指向进程环境变量指针数组（NULL 终止）。也通过 `weak_alias` 导出为 `environ`。