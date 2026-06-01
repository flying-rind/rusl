# getenv.c 规约

## 依赖图

```
getenv
├── __strchrnul  (跳过 — 来自 src/string/ 模块，内部符号)
├── strncmp      (跳过 — 来自 src/string/ 模块，标准 C 库函数)
└── __environ    (包含 — 同 src/env/ 模块，内部全局变量)
```

---

## __environ (内部全局变量)

```c
char **__environ;
```

[Visibility]: Internal — musl 内部全局变量，`__` 前缀约定表示不对外导出。实际通过 `weak_alias(__environ, environ)` 将 POSIX 标准名 `environ` 暴露给用户，但 `__environ` 本身不直接对用户可见。

**不变量 (Invariant):**
- 程序生命周期内，`__environ` 要么为 `NULL`（环境块尚未初始化），要么指向以 `NULL` 指针结尾的环境变量字符串数组。
- 每个环境变量字符串格式为 `"NAME=VALUE"`，其中 `NAME` 不包含 `=` 字符。
- 环境变量数组中的所有字符串指针均有效（指向进程合法内存）。

**初始化:**
```c
char **__environ = 0;
```
- 静态初始化为 `NULL` (0)。
- 实际的非 NULL 值由 C 运行时启动代码在调用 `main()` 之前设置（通常来自内核传递的 `envp` 参数或 `__environ` 在 `__libc_start_main.c` 中从 `envp` 参数填充）。

**weak_alias 绑定:**
```c
weak_alias(__environ, ___environ);
weak_alias(__environ, _environ);
weak_alias(__environ, environ);
```
- `environ` — POSIX 标准对外导出名，用户程序通过 `<unistd.h>` 中 `extern char **environ;` 声明使用。
- `___environ` / `_environ` — 兼容性别名，供不同 ABI/平台使用。

**前置条件 (读取):**
- 无（始终可读，返回当前环境指针或 NULL）。

**后置条件 (读取):**
- 返回指向环境变量数组的指针，或 `NULL`（环境尚未初始化）。
- 返回的指针在进程生命周期内保持有效，直至调用 `putenv`/`setenv`/`clearenv` 等修改环境的函数。

---

## getenv (对外导出)

```c
char *getenv(const char *name);
```

[Visibility]: Public — POSIX.1-2001 标准函数，`<stdlib.h>` 声明。用户程序可直接调用。

### 意图 (Intent)

在进程环境变量列表中查找指定名称的环境变量，返回其对应的值字符串。搜索使用精确名称匹配，遵循 POSIX 语义：名称不区分大小写并非此实现的要求（musl 实现严格区分大小写，符合 POSIX 标准允许的行为）。

### 前置条件 (Preconditions)

| 条件 | 说明 |
|------|------|
| `name != NULL` | 调用者必须传入有效的 C 字符串指针 |
| `name` 是以 `'\0'` 结尾的合法字符串 | 标准 C 字符串约束 |
| `name` 中不包含 `'='` 字符 | POSIX 标准规定：环境变量名不得包含 `=`；若 `name` 含 `=`，函数将返回 `NULL`（视为未找到） |
| `name` 长度 > 0 | 空字符串 `""` 不是合法的环境变量名，返回 `NULL` |

### 后置条件 (Postconditions)

**Case 1 — 找到匹配的环境变量:**

| 条件 | 结果 |
|------|------|
| `__environ != NULL` | 环境块已初始化 |
| 存在 `__environ[i]`，使得 `strncmp(name, __environ[i], l) == 0` 且 `__environ[i][l] == '='` | 名称匹配且后接 `=` 分隔符 |
| 返回值 | 指向 `__environ[i] + l + 1` 的指针，即值字符串的起始地址 |
| 返回值有效性 | 指向的字符串位于进程环境内存中，调用者**不得修改或释放**该内存 |
| 线程安全性 | musl 的 `getenv` 不持有锁，读操作本身是数据竞争安全的（读取 `char *` 指针），但若其他线程同时调用 `putenv`/`setenv`/`unsetenv`，行为未定义 |

**Case 2 — 未找到环境变量:**

| 条件 | 结果 |
|------|------|
| `__environ == NULL` | 环境块未初始化，直接返回 `NULL` |
| 或 `name` 为空字符串 (`l == 0`) | 返回 `NULL` |
| 或 `name` 含 `'='` 字符 (`name[l] != '\0'`) | 返回 `NULL`（被视为非法变量名） |
| 或遍历整个 `__environ` 数组后无匹配项 | 返回 `NULL` |
| 返回值 | `NULL` (`(char *)0`) |

### 系统算法 (System Algorithm)

```
function getenv(name):
    l := __strchrnul(name, '=') - name    // 计算 name 长度 (到第一个 '=' 或 '\0')
    if l == 0 or name[l] != '\0' or __environ == NULL:
        return NULL                        // 非法名称或环境未初始化
    
    for each e in __environ where *e != NULL:
        if strncmp(name, *e, l) == 0 and (*e)[l] == '=':
            return *e + l + 1             // 跳过 "NAME=" 前缀，返回值的起始地址
    
    return NULL
```

**算法要点:**
1. **名称校验**: 使用 `__strchrnul` 一次性完成长度计算与 `=` 字符检测，避免两次扫描。这是 musl 内部优化的字符串查找函数（一次遍历定位 `=` 或字符串末尾），比单独调用 `strlen` + `strchr` 更高效。
2. **惰性 NULL 检查**: `__environ` 在循环外检查一次，避免在每次迭代中重复检查。
3. **逐项线性扫描**: 对环境变量数组进行线性搜索，时间复杂度 O(n * m)，其中 n 为环境变量数目，m 为名称长度。POSIX 标准允许此实现复杂度。
4. **精确名称匹配**: 使用 `strncmp` 比较前 l 个字符，再检查第 l 个字符是否为 `=` —— 防止将 `NAME` 错误匹配到 `NAMEOTHER=value`。
5. **返回值语义**: 返回的是指向环境内存内部的指针，而非新分配的副本。调用者读取此指针是安全的，但不应修改或释放。

### 不变量 (Invariants)

- `getenv` 返回的指针在下次修改环境的调用（`putenv`、`setenv`、`unsetenv`、`clearenv`）之前保持有效；修改环境后，该指针可能指向已被替换或释放的内存。
- 若多次以相同 `name` 调用 `getenv` 且期间未修改环境，每次返回值相同。
- 该函数不设置 `errno`。