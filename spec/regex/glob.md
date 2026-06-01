# glob.c 规约

## 依赖图

```
glob ──────────────────────────> ignore_err, expand_tilde, do_glob, append, freelist, sort (via qsort)
globfree ──────────────────────> offsetof(struct match, name), free
do_glob ──┬────────────────────> append
          ├────────────────────> stat, lstat (外部)
          ├────────────────────> opendir, readdir, closedir (外部)
          ├────────────────────> strchr, strlen, memcpy (外部)
          ├────────────────────> fnmatch (外部)
          └────────────────────> do_glob (递归自身)
expand_tilde ──────────────────> __strchrnul (内部, see src/string/strchrnul.c), getenv, getpwnam_r, getpwuid_r, getuid (外部)
append ────────────────────────> malloc, memcpy (外部)
freelist ──────────────────────> free (外部)
sort ──────────────────────────> strcmp (外部)
```

---

## 内部类型定义

### struct match

```
struct match
{
    struct match *next;
    char name[];
};
```

**[Visibility]: Internal** — musl 内部数据结构，POSIX 标准未定义，仅在 `glob.c` 内使用，不对外导出。

**意图 (Intent)**:
连接匹配结果的单向链表节点。使用 C 灵活数组成员 (flexible array member) `name[]` 将文件名内联存储在节点分配中，避免额外的指针和二次分配。`name` 指针被直接存入 `glob_t.gl_pathv` 数组，而 `globfree` 通过 `offsetof(struct match, name)` 从 `name` 指针反推出原始 `malloc` 分配块的起始地址，从而正确释放整个节点。

**不变量 (Invariants)**:
- 链表头节点 (`head`) 是哑节点 (dummy)，其 `name` 字段为空字符串，`next` 指向第一个实际匹配项。
- 链表中所有非头节点的内存均通过 `malloc` 分配，其大小为 `sizeof(struct match) + name长度 + 1`（若 `GLOB_MARK` 设置且目标是目录则额外 +1 用于 '/' 后缀）。
- 每个节点 `name` 字段以 null 终止。

---

## 内部辅助函数（Static Functions）

### append

```c
static int append(struct match **tail, const char *name, size_t len, int mark);
```

**[Visibility]: Internal** — musl 内部辅助函数，POSIX 标准未定义。`static` 限定，仅在 `glob.c` 内使用。

**意图 (Intent)**:
向 match 链表尾部追加一个新的匹配项节点。若 `mark` 非零且文件名不以 '/' 结尾，则附加一个 '/' 后缀用于标识目录（`GLOB_MARK` 行为）。

**前置条件 (Preconditions)**:
- `tail` 非 NULL，且 `*tail` 指向链表当前尾部节点（初始时为哑头节点）。
- `name` 指向长度为 `len` 的以 null 终止的字符串（或至少 `len` 字节可读），不含尾部 '/'（除非外部传入时自带）。
- `(*tail)->next` 为 NULL（链表尾部不变量）。

**后置条件 (Postconditions)**:

| 分支 | 条件 | 返回值 | 状态变化 |
|------|------|--------|----------|
| Case 1 | `malloc` 失败 | `-1` | `*tail` 未修改，原链表不变。`errno` 设置为 `ENOMEM`。 |
| Case 2 | 分配成功 | `0` | 新节点挂接到 `(*tail)->next`，`*tail` 更新为新节点。新节点的 `name` 包含 `name` 的副本；若 `mark && len && name[len-1]!='/'`，则 `name[len]='/'` 且 `name[len+1]='\0'`。 |

**内存分配**:
分配大小为 `sizeof(struct match) + len + 2` 字节（+2 为：null 终止符 1 字节 + 可能的 '/' 后缀 1 字节）。

---

### do_glob

```c
static int do_glob(char *buf, size_t pos, int type, char *pat, int flags,
                   int (*errfunc)(const char *path, int err),
                   struct match **tail);
```

**[Visibility]: Internal** — musl 内部核心递归引擎，POSIX 标准未定义。`static` 限定，仅在 `glob.c` 内使用。

**意图 (Intent)**:
`glob` 的核心递归实现。逐级解析路径模式，将 wildcard 字符（`*`、`?`、`[...]`）匹配委托给 `fnmatch`，通过目录遍历 (`opendir/readdir`) 枚举候选文件，对每个匹配项递归进入下一级路径。返回 0 表示正常完成（可能无匹配），非零表示异常终止。

**系统算法 (System Algorithm)**:

`do_glob` 采用**逐级路径递归**策略处理 glob 模式匹配，整体流程如下：

1. **类型快速修正（Line 38）**: 若调用者未传入 `type` 且未要求 `GLOB_MARK`，则将 `type` 设为 `DT_REG`，避免后续不必要的 `stat` 调用。

2. **全是斜杠的边界情况处理（Lines 40-43）**: 若剩余模式全由 '/' 组成且 `type != DT_DIR`，将 `type` 清零以触发后续存在性检查；否则跳过所有前导 '/' 字符将 `pos` 推进。

3. **模式前缀逐字扫描（Lines 47-90）**:
   - 逐字节扫描模式 `pat`，当遇到 `*`、`?` 或括号外部的 `]` 时停止。
   - 转义处理：遇到 '\\' 时（`GLOB_NOESCAPE` 未设置），跳过反斜杠并取下一字符。括号内部的反斜杠 '\\' 后跟 ']' 时视为字面 ']'，终止扫描。
   - 未配对的结尾反斜杠（`pat[i+1]=='\0'`）直接返回 0（永不匹配）。
   - 遇到 '/' 时：重置 `in_bracket` 标志，将当前路径组件截断并写入 `buf`，`pos` 和 `j` 同步进位。
   - **溢出处理**: 若 `pos+(j+1) >= PATH_MAX` 时在括号内部遇到字符，设置 `overflow = 1` 标记。此标记允许长括号表达式继续解析——若括号最终闭合（即 `[` 是通配元字符），则使用 `fnmatch` 匹配整个组件，缓冲区溢出不影响匹配；若括号未闭合（即 `[` 是字面字符），则在 '/' 边界处返回 0。
   - 任何字符消费后将 `type` 清零，因为调用者传入的类型不再有效。

4. **路径组件消费完成后的处理（Lines 92-114）**:
   - 若 `*pat == '\0'`（模式耗尽）：执行终点处理。
     - `GLOB_MARK` 逻辑：若需要标记目录但类型未知或类型为 `DT_LNK`（符号链接），先用 `stat` 确定真实类型，若失败再回退到 `lstat` 确认存在性。
     - 若非目录类型且 `lstat` 失败：若错误不是 `ENOENT`，根据 `errfunc` 回调和 `GLOB_ERR` 标志决定返回 `GLOB_ABORTED` 或 0。
     - 调用 `append` 将累积路径写入匹配链表。

5. **含有通配符的组件的目录遍历（Lines 116-172）**:
   - 在剩余模式中定位 '/'，用于分隔当前组件与后续组件。校验 '/' 是否被奇数个反斜杠转义，若是则视为字面字符而非路径分隔符。
   - `opendir(pos ? buf : ".")` 打开当前累积路径所在的目录。
   - 遍历 `readdir` 返回的每个目录项：
     - 若后续还有路径组件 (`p2 != NULL`)，快速跳过非目录且非符号链接的条目。
     - 跳过名称过长导致路径超过 `PATH_MAX` 的条目。
     - 使用 `fnmatch(pat, de->d_name, fnm_flags)` 测试文件名是否匹配当前模式组件。
     - `GLOB_PERIOD` 处理：若设置了 `GLOB_PERIOD`，`.` 和 `..` 必须显式匹配模式（即模式必须以 '.' 开头）才能被接受。通过对比两次 `fnmatch` 调用（一次无 `FNM_PERIOD`，一次有 `FNM_PERIOD`）实现——若两者结果不同，说明该条目被 `FNM_PERIOD` 规则排除。
     - 将匹配的文件名复制到 `buf+pos`，递归调用 `do_glob` 处理下一级组件。
   - 读取目录出错时：根据 `errfunc` 回调和 `GLOB_ERR` 决定是否返回 `GLOB_ABORTED`。
   - 恢复原 `errno`（以防被 `closedir` 等操作覆盖）。

**前置条件 (Preconditions)**:
- `buf` 指向长度为 `PATH_MAX` 的字符数组；`buf[0..pos-1]` 包含当前累积的路径前缀（以 null 终止当需要作为路径时）。
- `pos < PATH_MAX`。
- `pat` 指向待匹配的剩余模式（应当已经过 `expand_tilde` 处理）。
- `type` 为调用者已知的 `dirent d_type` 值，或 0 表示未知。
- `flags` 为有效的 `GLOB_*` 标志组合。
- `errfunc` 非 NULL（调用者已在 `glob` 中提供默认值 `ignore_err`）。
- `tail` 与 `append` 中的约定一致。

**后置条件 (Postconditions)**:

| 分支 | 条件 | 返回值 | 状态变化 |
|------|------|--------|----------|
| Case 1 | 正常完成（可能有或无匹配） | `0` | `buf` 内容可能已被修改（路径临时构建）。匹配结果通过 `append` 写入 `*tail` 链表。 |
| Case 2 | 内存分配失败 | `GLOB_NOSPACE` | `errno == ENOMEM`。`buf` 内容未定义。递归过程中 `closedir` 确保目录描述符不泄漏。 |
| Case 3 | 目录操作错误且用户要求中止 | `GLOB_ABORTED` | 错误通过 `errfunc` 报告。`closedir` 已调用。 |

**不变量 (Invariants)**:
- `pos` 始终等于 `buf` 中累积路径的长度（不含 null 终止符）。
- `buf[pos]` 在调用 `do_glob` 入口处为 null 终止符，递归返回后仍为 null 终止符（临时修改在返回前恢复）。
- 每次 `opendir` 成功必有一次对应的 `closedir`，不存在目录描述符泄漏。
- 递归深度受路径深度自然限制（每个路径组件调用一次递归），不会被无限嵌套。

**跨文件依赖**:
- `fnmatch`: 来自 `src/regex/fnmatch.c` 的外部 POSIX 接口，执行模式匹配。
- `__strchrnul` 未在 `do_glob` 内直接使用，被 `expand_tilde` 依赖。

---

### ignore_err

```c
static int ignore_err(const char *path, int err);
```

**[Visibility]: Internal** — musl 内部默认错误处理函数，POSIX 标准未定义。`static` 限定，仅在 `glob.c` 内使用。

**意图 (Intent)**:
当用户未提供自定义 `errfunc` 时，作为 `glob` 的默认错误处理回退。该函数忽略所有错误，使得 glob 在遭遇不可访问目录时静默跳过，这是 POSIX 的默认行为。

**前置条件 (Preconditions)**:
- `path` 为触发错误的路径（可 NULL）。
- `err` 为 `errno` 值。

**后置条件 (Postconditions)**:
- 始终返回 `0`（非零返回值会触发 `glob` 中止）。

---

### sort

```c
static int sort(const void *a, const void *b);
```

**[Visibility]: Internal** — musl 内部比较函数，POSIX 标准未定义。`static` 限定，仅作为 `qsort` 的回调使用。

**意图 (Intent)**:
`qsort` 的比较函数，对 `char *` 指针数组（`glob_t.gl_pathv`）进行字典序排序。参数 `a` 和 `b` 是指向 `char *` 元素的指针（即 `const char **`），需解引用后传给 `strcmp`。

**前置条件 (Preconditions)**:
- `a` 和 `b` 各指向 `glob_t.gl_pathv` 数组中的有效 `char *` 元素。

**后置条件 (Postconditions)**:
- 返回 `strcmp(*(const char **)a, *(const char **)b)` 的结果：负值若 `a < b`，零若 `a == b`，正值若 `a > b`。

---

### freelist

```c
static void freelist(struct match *head);
```

**[Visibility]: Internal** — musl 内部清理函数，POSIX 标准未定义。`static` 限定，仅在 `glob.c` 内使用。

**意图 (Intent)**:
释放整个 match 链表从 `head` 节点（不包括 `head` 本身，因其为栈上分配的哑节点）开始的所有节点。`head` 必须是通过 `glob` 中栈上的 `struct match head` 哑节点传入。

**前置条件 (Preconditions)**:
- `head` 非 NULL，是哑头节点（栈上分配），其 `next` 可能指向实际匹配节点链（堆分配）或为 NULL。
- 链表中的所有堆节点均为 `append` 通过 `malloc` 分配。

**后置条件 (Postconditions)**:
- 所有 `head->next` 可达的堆分配节点均已 `free`。
- `head` 本身未被释放（调用者栈上变量）。

---

### expand_tilde

```c
static int expand_tilde(char **pat, char *buf, size_t *pos);
```

**[Visibility]: Internal** — musl 内部波浪号展开函数，POSIX 标准未定义。`static` 限定，仅在 `glob.c` 内使用。

**意图 (Intent)**:
将 glob 模式开头的 `~` 或 `~username/` 展开为对应的用户家目录路径。支持三种形式：`~/` 展开为当前用户家目录；`~user/` 展开为指定用户家目录；无结尾 '/' 时通过 `HOME` 环境变量获得当前用户家目录，或通过密码数据库查找。

**系统算法 (System Algorithm)**:

1. **解析用户名**: 使用 `__strchrnul(p, '/')` 定位 `~` 后的用户名边界（'/' 或 null 终止符）。记录分隔符 `delim`，同时推进 `*pat` 跳过用户名字段。
2. **获取家目录**: 若用户名为空（仅 `~`），通过 `getenv("HOME")` 获取。若 `HOME` 为空，回退到 `getpwuid_r(getuid(), ...)` 查询当前用户密码条目。
3. 若用户名非空，通过 `getpwnam_r` 查询指定用户密码条目。
4. 若密码查询返回 `ENOMEM`，返回 `GLOB_NOSPACE`；若用户不存在或系统错误，返回 `GLOB_NOMATCH`。
5. **写入缓冲区**: 将家目录路径逐字节复制到 `buf`，长度不得超过 `PATH_MAX - 2`。若家目录过长，返回 `GLOB_NOMATCH`。
6. 若原始模式中有分隔符 '/'，追加到缓冲区末尾。
7. 更新 `*pos` 为已写入字节数。

**前置条件 (Preconditions)**:
- `*pat` 指向以 '~' 字符开头的模式字符串（调用者已通过 `(*p == '~')` 判断）。
- `buf` 指向长度为 `PATH_MAX` 的字符数组。
- `*pos` 为缓冲区当前写入偏移。
- `flags` 包含 `GLOB_TILDE` 或 `GLOB_TILDE_CHECK`（由 `glob` 调用者保证）。

**后置条件 (Postconditions)**:

| 分支 | 条件 | 返回值 | 状态变化 |
|------|------|--------|----------|
| Case 1 | 家目录展开成功 | `0` | `buf[0..*pos-1]` 包含家目录路径（若有分隔符则 `buf[*pos-1]=='/'`），`buf[*pos]` 为 null 终止符。`*pat` 指向跳过用户名和分隔符后的剩余模式。 |
| Case 2 | 内存不足（`getpwnam_r`/`getpwuid_r` 返回 `ENOMEM`） | `GLOB_NOSPACE` | `buf` 内容未定义。 |
| Case 3 | 用户或家目录无法确定 | `GLOB_NOMATCH` | `buf` 内容未定义。 |

**跨文件依赖**:
- `__strchrnul`: 来自 `src/string/strchrnul.c`，musl 内部函数，类似 `strchr` 但在未找到字符时返回指向 null 终止符的指针而非 NULL。

---

## 对外导出函数（Public API）

### glob

```c
int glob(const char *restrict pat, int flags,
         int (*errfunc)(const char *path, int err),
         glob_t *restrict g);
```

**[Visibility]: Public** — POSIX.1-2001 标准函数，在 `<glob.h>` 中声明。用户程序可直接调用。

**意图 (Intent)**:
根据 shell 使用的规则，查找文件系统中匹配指定模式 `pat` 的路径名。支持通配字符 (`*`, `?`, `[...]`)、波浪号展开 (`~`)、目录标记、定制错误处理等功能。结果存储于 `glob_t` 结构中。与 GNU glob 扩展兼容（`GLOB_TILDE`、`GLOB_TILDE_CHECK`）。

**系统算法 (System Algorithm)**:

1. **初始化**: 创建栈上的哑头节点 `head`（`{ .next = NULL }`），`tail` 指向 `&head`。若用户未提供 `errfunc`，默认为 `ignore_err`。
2. **非追加模式重置**: 若未设置 `GLOB_APPEND`，重置 `g` 的 `gl_offs`、`gl_pathc`（设为 0）、`gl_pathv`（设为 NULL）。
3. **模式的拷贝与预处理**: 若 `*pat != '\0'`：
   - `strdup(pat)` 拷贝模式字符串（因为后续 `do_glob` 需在解析过程中临时修改模式字符串）。
   - 若 `GLOB_TILDE` 或 `GLOB_TILDE_CHECK` 标志置位且模式以 '~' 开头，调用 `expand_tilde` 展开波浪号。
   - 调用 `do_glob(buf, pos, 0, s, flags, errfunc, &tail)` 执行核心匹配。
   - `free(p)` 释放模式拷贝。
4. **内存不足提前退出**: 若 `error == GLOB_NOSPACE`，释放链表并返回 `GLOB_NOSPACE`。
5. **计数与空结果处理**: 遍历链表统计匹配数量 `cnt`。
   - 若 `cnt == 0` 且设置了 `GLOB_NOCHECK`：将原模式作为唯一的匹配项追加（字面返回模式字符串）。
   - 若 `cnt == 0` 且未出错：返回 `GLOB_NOMATCH`。
6. **分配结果数组**: 若设置了 `GLOB_APPEND`，使用 `realloc` 扩展已有 `gl_pathv`；否则 `malloc` 新数组。数组大小为 `(offs + cnt + 1) * sizeof(char *)`（+1 用于尾部的 NULL 哨兵）。
7. **填充结果**: 遍历链表，将每个节点的 `name` 指针填入 `gl_pathv[offs + i]`，最后一个槽位置为 NULL。
8. **排序**: 若未设置 `GLOB_NOSORT`，使用 `qsort` 按字典序排序结果。
9. **返回**: 返回首个错误码（若有）或 0。

**前置条件 (Preconditions)**:
- `pat` 为以 null 终止的合法 glob 模式字符串（不能为 NULL）。
- `g` 为非 NULL 的 `glob_t` 指针。
- 若 `flags & GLOB_APPEND`，`g` 应为先前有效 `glob` 调用的结果（`gl_pathv` 为有效指针或 NULL），且 `gl_offs` 已正确设置。
- `flags` 为 `GLOB_*` 标志的有效按位或组合。`GLOB_DOOFFS` 与 `GLOB_APPEND` 同时使用时，`gl_offs` 在首次调用中指定。

**后置条件 (Postconditions)**:

| 分支 | 条件 | 返回值 | 状态变化 |
|------|------|--------|----------|
| Case 1 | 匹配成功（至少一个匹配项） | `0`（若无错误）或首个 `GLOB_ABORTED` 后的 error | `g->gl_pathc` 为匹配数；`g->gl_pathv[0..gl_offs-1]` 为 NULL（若 `GLOB_DOOFFS`）；`g->gl_pathv[gl_offs..gl_offs+gl_pathc-1]` 为匹配路径字符串指针；`g->gl_pathv[gl_offs+gl_pathc]` 为 NULL。未设置 `GLOB_NOSORT` 时结果已排序。`gl_pathv` 各指针指向 `struct match` 的 `name` 字段，可通过 `offsetof` 反推原始分配块。 |
| Case 2 | 无匹配且无 `GLOB_NOCHECK` | `GLOB_NOMATCH` | `g->gl_pathc` 为 0；若未设置 `GLOB_APPEND`，`gl_pathv` 为 NULL。 |
| Case 3 | 无匹配但设置 `GLOB_NOCHECK` | `0`（若无错误） | `g->gl_pathv[gl_offs]` 指向原 `pat` 的副本；`g->gl_pathc` 为 1。 |
| Case 4 | 内存分配失败 | `GLOB_NOSPACE` | `errno == ENOMEM`。`g` 状态：若分配发生在 `realloc` 之前，`g` 保持原值不变（包括 `GLOB_APPEND` 下的原有数据）；已分配的 match 链表已释放。 |
| Case 5 | 目录访问错误且需要中止 | `GLOB_ABORTED` | `g->gl_pathv` 可能包含部分结果（若 `GLOB_APPEND` 下多目录匹配时某个后续目录失败），但调用者应检查返回值。 |

**资源管理契约**:
- 调用者必须通过 `globfree(g)` 释放成功调用（返回值为 0 或 `GLOB_NOMATCH` 分配了 `gl_pathv` 的情况下）所分配的资源。
- 返回 `GLOB_NOSPACE` 时，`glob` 内部已释放所有临时分配，`g` 保持调用前状态。

---

### globfree

```c
void globfree(glob_t *g);
```

**[Visibility]: Public** — POSIX.1-2001 标准函数，在 `<glob.h>` 中声明。用户程序可直接调用。

**意图 (Intent)**:
释放由先前 `glob()` 调用在 `glob_t` 结构中分配的所有内存。通过 `offsetof(struct match, name)` 从 `gl_pathv` 中各 `char *` 指针反推出原始 `struct match` 分配块的起始地址，从而正确释放每个路径字符串所属的内存块，最后释放 `gl_pathv` 数组本身。

**系统算法 (System Algorithm)**:

逐项释放方法（利用 C 结构体内存布局）：
1. 对 `i = 0` 到 `g->gl_pathc - 1`：取 `g->gl_pathv[g->gl_offs + i]`，减去 `offsetof(struct match, name)` 得到 `struct match` 节点在堆上的起始地址，调用 `free` 释放整个节点。
2. 释放 `g->gl_pathv` 数组本身。
3. 重置 `g->gl_pathc = 0` 和 `g->gl_pathv = NULL`，使 `g` 回到初始状态。

**前置条件 (Preconditions)**:
- `g` 非 NULL，且其内容由先前的 `glob()` 调用产生（或已初始化为全零/`globfree` 重置后的状态）。
- 多次调用 `globfree` 对同一 `g` 是安全的——第二次调用时 `gl_pathc == 0` 且 `gl_pathv == NULL`，`for` 循环零次执行。

**后置条件 (Postconditions)**:
- 所有 `glob()` 为 `g` 分配的内存均已释放（包括各路径字符串的 `struct match` 节点和 `gl_pathv` 数组）。
- `g->gl_pathc == 0`，`g->gl_pathv == NULL`。
- 无返回值。函数不会失败。
