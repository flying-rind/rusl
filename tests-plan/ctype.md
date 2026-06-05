# ctype 模块测试

## 提取自 libc-test

### src/api/ — API 接口测试 (编译时测试)

- `ctype.c` — 测试以下函数签名: isalnum, isalpha, isascii, isblank, iscntrl, isdigit, isgraph, islower, isprint, ispunct, isspace, isupper, isxdigit, toascii, tolower, toupper 及对应的 `_l` 变体

### src/functional/ — 功能测试

ctype 模块在 functional 目录中无对应功能测试。

### src/regression/ — 回归测试

ctype 模块在 regression 目录中无对应回归测试。

---

**总计**: ctype 模块在 libc-test 中仅有 1 个 API 接口测试文件 (`src/api/ctype.c`)，为编译时签名检查。无运行时功能测试或回归测试。