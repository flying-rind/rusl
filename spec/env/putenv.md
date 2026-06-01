# putenv.c 规约

## 依赖图

```
putenv (Public)
  ├── __strchrnul    → see src/string/__strchrnul.c spec (跨模块)
  ├── unsetenv       → see src/env/unsetenv.c spec (跨模块)
  └── __putenv (Internal)
        ├── __environ     → see src/env/__environ.c spec (跨模块)
        ├── strncmp       → <string.h>，外部 libc，跳过
        ├── malloc        → <stdlib.h>，外部 libc，跳过
        ├── realloc       → <stdlib.h>，外部 libc，跳过
        ├── free          → <stdlib.h>，外部 libc，跳过
        ├── memcpy        → <string.h>，外部 libc，跳过
        └── __env_rm_add  → 弱符号，默认 dummy / setenv.c 覆盖 (Internal)
              └── dummy (static, 空实现)
```

---

## dummy (内部辅助函数)

```c
static void dummy(char *old, char *new) {}
```

[Visibility]: Internal -- `static` 函数，不对外导出

### 意图 (Intent, Level 2)

`dummy` 是一个空占位函数，作为 `__env_rm_add` 弱符号的默认实现。它的语义是"什么都不做"：当 musl 的 `putenv` / `unsetenv` 被调用且 `setenv` 模块未被链接时（即链接时未解析到 `setenv.c` 中的强定义），环境变量被替换/移除时不需要额外的内存管理 Hook。这使得 `putenv` 和 `unsetenv` 模块可独立链接，而不会因缺少 `__env_rm_add` 符号而链接失败。

### 前置条件

- 无前置条件。`old` 和 `new` 可以为任意值（包括 NULL），函数体为空，不访问参数。

### 后置条件

- 无任何副作用，返回值类型为 `void`，不做任何操作。

---

## __env_rm_add (弱符号，内部 Hook)

```c
weak_alias(dummy, __env_rm_add);
void __env_rm_add(char *old, char *new);
```

[Visibility]: Internal -- musl 内部弱符号，非 POSIX/C 标准接口，用于模块间通信。`setenv.c` 提供强定义覆盖此弱符号；若 `setenv` 未被链接，则使用 `dummy` 空实现。

### 意图 (Intent, Level 2)

`__env_rm_add` 是 musl 环境变量模块的内部通信协议：它是一个可选的弱 Hook，用于在环境变量被替换（`putenv`/`__putenv`）或移除（`unsetenv`）时通知内存管理模块。`setenv.c` 中的强定义追踪所有通过 `setenv` 分配的环境字符串，以便在后续 `setenv` 或 `unsetenv` 调用时正确释放（`free(old)`）和更新跟踪表。若调用方只使用 `putenv` 和 `unsetenv`（不涉及 `setenv` 的堆分配），则该 Hook 无需执行任何操作。

### 前置条件

- `old`: 指向被替换/移除的旧环境变量字符串，或为 NULL（当新增变量时）
- `new`: 指向新分配的环境变量字符串，或为 NULL（当移除变量时）

### 后置条件

**弱定义（dummy，默认行为）:**
- 无操作，忽略 `old` 和 `new`

**强定义（setenv.c，当 setenv 模块被链接时）:**
- 若 `old != NULL` 且在跟踪表中，则：将其替换为 `new`，并 `free(old)`
- 若 `old` 不在跟踪表中且 `new != NULL`：将 `new` 添加到跟踪表，以便后续释放
- 若 `old` 不在跟踪表中且 `new == NULL`：无操作（`old` 不是由 `setenv` 分配的，无需释放）
- 不变量：跟踪表 `env_alloced[]` 始终只包含当前有效的、由 `setenv` 分配的环境字符串指针

---

## __putenv (内部函数)

```c
int __putenv(char *s, size_t l, char *r);
```

[Visibility]: Internal -- `__` 前缀的 musl 内部函数，POSIX/C 标准未定义。被 `putenv` 和 `setenv` 调用。

### 意图 (Intent, Level 2)

`__putenv` 是环境变量设置的核心实现。它将字符串 `s` 插入进程的 `__environ` 数组（若 `s` 对应的变量已存在则原地替换），并在必要时扩增 `__environ` 数组的容量。参数 `r` 用于内存管理：若调用方（如 `setenv`）在堆上分配了新字符串，`r` 指向该新字符串；`putenv` 则传入 `r = 0` 表示无需额外内存管理。当环境数组扩容成功但 `r != NULL` 时，`__env_rm_add(0, r)` 被调用以将新字符串注册到跟踪表。

### 参数说明

| 参数 | 类型       | 含义 |
|------|------------|------|
| `s`  | `char *`   | 指向 `"NAME=VALUE"` 格式的字符串，将被直接放入环境数组（非拷贝） |
| `l`  | `size_t`   | 环境变量名的长度（不含 `=`），即 `=` 在 `s` 中的偏移量 |
| `r`  | `char *`   | 若调用方在堆上分配了字符串，则为指向该内存的指针；否则为 `0` / NULL |

### 前置条件

- `s` 必须指向一个有效的、以 `=` 分隔的 `"NAME=VALUE"` 字符串，且 `s[l] == '='`
- `l > 0`（变量名非空）
- `__environ` 可能是 NULL（表示环境变量数组尚未初始化）或指向以 NULL 结尾的 `char *` 数组
- `s` 指向的内存生命期必须不短于其在环境数组中的存留时间（调用方负责管理）

### 后置条件

**Case 1: 变量已存在（成功替换）**

- 返回值: `0`
- 在 `__environ` 中查找第一个 `*e` 满足 `strncmp(s, *e, l+1) == 0`（即前 `l+1` 个字符匹配，含 `=`）
- 将 `*e` 替换为 `s`（原地替换，不改变数组大小）
- 调用 `__env_rm_add(tmp, r)` 通知旧值 `tmp` 被替换为新值 `r`（对于 `putenv` 调用路径，`r=0`，通知模块旧值被移除；对于 `setenv` 路径，同时注册 `r`）

**Case 2: 变量不存在（需要插入）**

- 返回值: `0` 成功，`-1` 失败（OOM）
- 计算新数组大小: `i+2`（`i` 个现有变量 + 1 个新变量 + 1 个 NULL 终止符）
- 若 `__environ == oldenv`（上次由 `__putenv` 分配），使用 `realloc` 扩容，失败则跳转 OOM
- 否则使用 `malloc` 分配新数组，若 `i > 0` 则 `memcpy` 旧内容，释放 `oldenv`
- 将 `newenv[i] = s`，`newenv[i+1] = 0`，更新 `__environ = oldenv = newenv`
- 若 `r != NULL`，调用 `__env_rm_add(0, r)` 注册新分配的字符串
- OOM 路径: `free(r)`（若 `r` 非 NULL，释放调用方传入的堆分配字符串），返回 `-1`

### 不变量 (Invariants)

- **`oldenv` 追踪**: `static char **oldenv` 记录上一次由 `__putenv` 分配的数组指针。若当前 `__environ == oldenv`，说明当前环境数组由此模块管理，可使用 `realloc`；否则说明 `__environ` 指向外部传入的数组（如 `execve` 传入的 `envp`），需新分配。
- **NULL 终止**: 环境数组始终以 NULL 指针终止，即 `__environ` 指向的数组的最后一个有效元素之后是 `NULL`。
- **`oldenv` 一致性**: 每次通过 `malloc`/`realloc` 分配新数组后，`oldenv` 始终等于 `__environ`，确保下次插入时能正确识别为"自管理"数组而使用 `realloc`。

---

## putenv (对外导出)

```c
int putenv(char *s);
```

[Visibility]: Public -- POSIX 标准函数，`<stdlib.h>` 声明，用户程序可直接调用。

### 系统算法 (System Algorithm, Level 3)

`putenv` 的职责是将调用方提供的 `"NAME=VALUE"` 格式字符串直接放入进程环境（非拷贝）。POSIX 标准规定：调用方不得在 `putenv` 后修改或释放 `s`，且该字符串将作为环境的一部分，直到被后续 `putenv` / `setenv` 覆盖或 `unsetenv` 移除。

完整的执行流程：

1. **解析变量名长度**: 调用 `__strchrnul(s, '=')` 查找 `=` 的位置，计算 `l = 偏移量 = 变量名长度`
2. **有效性检查**: 若 `l == 0`（空变量名）或 `s[l] == '\0'`（无 `=`），则该字符串不符合 `"NAME=VALUE"` 格式，委托给 `unsetenv(s)` 处理（POSIX 允许此行为：若字符串不包含 `=`，则视为移除同名环境变量）
3. **委托核心逻辑**: 调用 `__putenv(s, l, 0)`，其中 `r = 0` 表示 `putenv` 未在堆上分配字符串，无需 `__env_rm_add` 注册

### 参数说明

| 参数 | 类型       | 含义 |
|------|------------|------|
| `s`  | `char *`   | 指向 `"NAME=VALUE"` 格式的字符串，调用方必须保证其在整个环境存续期间有效且不被修改 |

### 前置条件

- `s` 不为 NULL，且指向以 null 结尾的 C 字符串
- 若 `s` 包含 `=`，则 `=` 前至少有一个字符（即 `s[0] != '='`），否则视为无效，触发 `unsetenv` 行为
- 调用方在 `putenv` 返回后不得修改或释放 `s` 指向的内存，除非之后通过另一次 `putenv`/`setenv` 覆盖或 `unsetenv` 移除了该变量
- 进程的环境变量数组（`environ`/`__environ`）可能处于未初始化状态或已初始化状态

### 后置条件

**Case 1: 设置/替换环境变量（`s` 含 `=`）**

- 返回值: `0`
- `s` 被直接放入环境变量数组（非拷贝），`__environ` 中对应条目指向 `s`
- 若同名变量已存在，旧值被 `s` 取代
- 若同名变量不存在，环境数组扩容并追加 `s`

**Case 2: 移除环境变量（`s` 不含 `=` 或变量名为空）**

- 返回值: `unsetenv(s)` 的返回值
  - `0` 成功（变量被移除或本就不存在）
  - `-1` 且 `errno = EINVAL`（`s` 中包含 `=` 后的字符，即 `l > 0` 但 `s[l] != '\0'`，例如 `s = "A"` 之后的字符非空——实际上此路径几乎不可能触发，因为 `s[l] == '\0'` 时 `putenv` 本身已检测到无 `=` 并调用 `unsetenv`）

**错误路径 (OOM)**

- `__putenv` 内部若 `malloc`/`realloc` 失败，返回 `-1`，此时 `s` 未被添加到环境，原环境不变

### 不变量 (Invariants)

- `putenv` 不拥有 `s` 的内存所有权，因此不负责释放 `s`
- `putenv` 调用通过 `r = 0` 告知 `__env_rm_add` 不注册新内存，这与 `setenv`（传入 `r = s`，注册堆分配内存）形成对比

---

## 与 setenv / unsetenv 的协作语义

`putenv.c`、`setenv.c`、`unsetenv.c` 三个模块共享 `__env_rm_add` 弱符号机制实现协同：

- **putenv**: 调用 `__putenv(s, l, 0)`，`r=0`，不注册内存。当替换/移除时调用 `__env_rm_add(old, 0)` 通知旧值被移除。
- **setenv**: 先 `malloc` 构造 `"NAME=VALUE"` 字符串，再调用 `__putenv(s, l, s)`，`r=s`，在插入成功后通过 `__env_rm_add(0, r)` 注册。替换时调用 `__env_rm_add(old, s)` 释放旧值并注册新值。
- **unsetenv**: 直接遍历 `__environ`，对匹配项调用 `__env_rm_add(*e, 0)` 通知释放，并压缩数组。

这种设计使得 `setenv` 分配的堆内存能在被覆盖或移除时正确释放，而 `putenv` 传入的用户内存不受影响。