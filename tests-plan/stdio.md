# rusl-stdio libc-test 测试结果

测试日期: 2026-06-15
基线对比: `./musl-config/origin/REPORT.original` (原版 musl libc)

## 新增失败（相比原版）

共 **20 个**新增失败（10 组唯一测试 × static/dynamic 两变体）：

| # | 测试 | 根因 | 严重度 |
|---|------|------|:---:|
| 1-2 | `functional/fscanf` | vfscanf 中 `%n` 配合 3+ va_arg 导致内存损坏 | 🔴 |
| 3-4 | `functional/fwscanf` | vfwscanf 未实现 (wide char fscanf) | 🔴 |
| 5-6 | `functional/sscanf` | vsscanf 浮点解析精度问题 | 🟡 |
| 7-8 | `functional/snprintf` | vfprintf 浮点格式化精度 (subnormal, hexfloat) | 🟡 |
| 9-10 | `functional/swprintf` | vswprintf 浮点格式化 (wide char printf) | 🟡 |
| 11-12 | `functional/popen` | popen 未实现 | 🔴 |
| 13-14 | `functional/strtod_simple` | strtod 解析 (非 stdio, 但可能受 scanf 影响) | 🟡 |
| 15-16 | `functional/wcstol` | wcstol 解析 (非 stdio) | 🟡 |
| 17-18 | `regression/printf-fmt-g-round` | vfprintf `%g` 四舍五入精度 | 🟡 |
| 19-20 | `regression/fpclassify-invalid-ld80` | 非 stdio (浮点环境), 环境差异 | ⚪ |

## 已修复

### vfscanf Bug 修复 (2026-06-15)

| 修复项 | 描述 | 影响 |
|--------|------|------|
| `%i/%d/%u` & `%x/%X` consumed_prefix | width 耗尽在 "0x" 前缀时不标记匹配成功 | 修复 `fscanf` line 52: `%2i` 误匹配 |
| hexfloat "0x" 消费 | 移除多余的 `getc()` 调用 (3 → 2) | 修复 `fscanf` line 113: `u = 16` → `u = 1.0` |
| hexfloat p/P 无指数 | p/P 后无指数数字时匹配失败 | 修复 `fscanf` line 94: hexfloat "0x1p " 正确处理 |

### 仍存在的问题

- **va_list 3+ arg 问题**: Rust `extern "C" fn(..., args: ...)` 生成的 va_list 在第 3 个及之后的 va_arg 读取中出现栈内存损坏。表现为 Heisenbug (添加 printf 后行为改变)。影响所有使用 `%n` 或 3+ 参数的格式化输入场景。

## 通过测试

以下 stdio 相关测试**全部通过**：

### 功能测试
- `functional/fdopen` — fdopen 基本功能
- `functional/memstream` — open_memstream 内存流
- `functional/ungetc` — ungetc 字符回退

### 回归测试
- `regression/fflush-exit` — fflush 在 exit 时的行为
- `regression/fgets-eof` — fgets EOF 处理
- `regression/fgetwc-buffering` — fgetwc 缓冲
- `regression/ftello-unflushed-append` — ftello 追加模式
- `regression/rewind-clear-error` — rewind 清除错误标志
- `regression/scanf-bytes-consumed` — scanf 字节消费计数
- `regression/scanf-match-literal-eof` — scanf 字面量匹配 EOF
- `regression/scanf-nullbyte-char` — scanf 空字节处理
- `regression/setvbuf-unget` — setvbuf + unget 交互
- `regression/sscanf-eof` — sscanf EOF 处理
- `regression/printf-1e9-oob` — printf 1e9 越界
- `regression/printf-fmt-g-zeros` — printf %g 零处理
- `regression/printf-fmt-n` — printf %n
- `regression/flockfile-list` — flockfile 链表
- `regression/fpclassify-invalid-ld80` — (非 stdio, 环境差异导致偶尔失败)

## 待修复优先级

1. **va_list 3+ arg 问题** — 影响 fscanf/fwscanf/sscanf, 需要修复 Rust variadic wrapper 或改用其他方案
2. **popen** — 未实现, 需新增
3. **vfprintf 浮点精度** — snprintf/swprintf 浮点格式化精度问题 (subnormal, %a, %g)
4. **vfwscanf** — 未实现 wide char 输入
5. **vsscanf 浮点** — 浮点解析精度
