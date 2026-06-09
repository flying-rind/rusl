# musl malloc 模块相关测试文件

## API 测试

- `src/api/stdlib.c` — malloc, calloc, realloc, free, reallocarray, memalign, posix_memalign, aligned_alloc, malloc_usable_size 函数签名与常量检查

## 回归测试

- `src/regression/malloc-0.c` — malloc(0) 行为：应返回可安全传给 free() 的非 NULL 唯一指针
- `src/regression/malloc-oom.c` — malloc 在内存耗尽时正确返回 NULL 并设置 errno=ENOMEM
- `src/regression/malloc-brk-fail.c` — brk() 系统调用失败时，malloc 应回退到 mmap() 完成分配

## 间接测试（使用 malloc 作为辅助分配器）

这些测试不直接测试 malloc API，但内部依赖 `malloc`/`free`/`realloc` 进行动态内存分配，
malloc 实现的质量直接决定这些测试能否通过：

- `src/api/glob.c` — glob 模式匹配内部分配
- `src/api/locale.c` — locale 对象内部分配
- `src/api/netdb.c` — 网络地址数据库内部分配
- `src/api/net_if.c` — 网络接口查找内部分配
- `src/api/regex.c` — 正则表达式编译内部分配
- `src/api/sys_statvfs.c` — 文件系统状态内部分配
- `src/api/wordexp.c` — 单词扩展内部分配
- `src/functional/memstream.c` — open_memstream 依赖 malloc
- `src/functional/search_insque.c` — insque/remque 内部分配
- `src/functional/sscanf_long.c` — sscanf 长字符串内部分配
- `src/regression/flockfile-list.c` — flockfile 链表内部分配
- `src/regression/putenv-doublefree.c` — double-free 检测
- `src/regression/regexec-nosub.c` — regex 执行内部分配
- `src/regression/statvfs.c` — statvfs 内部分配

## rusl-malloc 测试结果 (基于 musl-1.2.6 libc + librusl_malloc.a)

| 测试 | 状态 | 说明 |
|------|:--:|------|
| `src/regression/malloc-0` | ✅ PASS | malloc(0) 返回非空唯一指针 |
| `src/regression/malloc-oom` | ✅ PASS | OOM 时正确返回 NULL + ENOMEM |
| `src/regression/malloc-brk-fail` | ❌ FAIL | brk() 失败后 mmap 回退逻辑与 musl 不同 |
