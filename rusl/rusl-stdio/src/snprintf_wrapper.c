// snprintf — C varargs wrapper for Rust vsnprintf
//
// Rust 不支持稳定的 C 可变参数 (va_list), 因此 snprintf 由
// 本 C 文件实现, 内部调用 Rust 实现的 vsnprintf。

#include <stdarg.h>

// 由 Rust rusl-stdio 模块提供
int vsnprintf(char *buf, unsigned long size, const char *fmt, va_list ap);

int snprintf(char *buf, unsigned long size, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int ret = vsnprintf(buf, size, fmt, ap);
    va_end(ap);
    return ret;
}