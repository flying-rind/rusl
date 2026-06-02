# libc-test 测试模块分类

libc-test 是 musl libc 项目的测试套件。每个测试文件包含自己的 `main` 函数，返回 0 表示成功，非 0 表示失败。

---

## 目录结构总览

| 目录 | 类型 | 构建方式 | 说明 |
|------|------|----------|------|
| `src/api/` | API 接口测试 | 编译链接到单一 `main.exe` | 编译时检查头文件中的函数签名和常量 |
| `src/common/` | 公共工具库 | 编译为 `libtest.a` | 测试框架基础库，不直接测试 |
| `src/functional/` | 功能测试 | 每个 .c 编译为独立可执行文件 | 运行时功能测试，覆盖面广 |
| `src/math/` | 数学函数测试 | 每个 .c 编译为独立可执行文件 | 每个数学函数带输入/输出测试向量 |
| `src/regression/` | 回归测试 | 每个 .c 编译为独立可执行文件 | 针对特定 bug 修复的回归测试 |
| `src/musl/` | musl 内部测试 | 编译为独立可执行文件 | 测试 musl 内部 API |

---

## 一、src/api/ — API 接口测试 (80 个文件)

API 测试是**编译时**测试，检查头文件中的函数声明、常量、类型定义是否正确。每个文件包含一个头文件的函数指针赋值，编译通过即表示签名正确。所有文件链接为单一 `main.exe`。

### 1.1 字符处理 (ctype.h)
- `ctype.c` — isalnum, isalpha, isascii, isblank, iscntrl, isdigit, isgraph, islower, isprint, ispunct, isspace, isupper, isxdigit, toascii, tolower, toupper 及对应的 _l 变体

### 1.2 字符串处理 (string.h, strings.h)
- `string.c` — memchr, memcmp, memcpy, memmove, memset, strcat, strchr, strcmp, strcoll, strcpy, strcspn, strerror, strlen, strncat, strncmp, strncpy, strpbrk, strrchr, strsignal, strspn, strstr, strtok, strxfrm, stpcpy, stpncpy, strdup, strndup, strnlen, strtok_r, strerror_r, strcoll_l, strerror_l
- `strings.c` — ffs, strcasecmp, strncasecmp, index, rindex

### 1.3 标准库 (stdlib.h)
- `stdlib.c` — _Exit, abort, abs, atexit, atof, atoi, atol, atoll, bsearch, calloc, div, exit, free, getenv, labs, ldiv, llabs, lldiv, malloc, mblen, mbstowcs, mbtowc, mktemp, putenv, qsort, rand, realloc, srand, strtod, strtof, strtol, strtold, strtoll, strtoul, strtoull, system, wcstombs, wctomb, 及 POSIX 扩展

### 1.4 标准IO (stdio.h)
- `stdio.c` — clearerr, ctermid, dprintf, fclose, fdopen, feof, ferror, fflush, fgetc, fgetpos, fgets, fileno, flockfile, fopen, fprintf, fputc, fputs, fread, freopen, fscanf, fseek, fsetpos, ftell, ftrylockfile, funlockfile, fwrite, getc, getchar, getdelim, getline, gets, getw, perror, popen, printf, putc, putchar, puts, remove, rename, rewind, scanf, setbuf, setvbuf, snprintf, sprintf, sscanf, tmpfile, tmpnam, ungetc, vdprintf, vfprintf, vfscanf, vprintf, vscanf, vsnprintf, vsprintf, vsscanf, pclose

### 1.5 数学 (math.h, complex.h, fenv.h, float.h, tgmath.h)
- `math.c` — 数学函数签名和常量
- `complex.c` — 复数类型和函数
- `fenv.c` — 浮点环境
- `float.c` — 浮点常量
- `tgmath.c` — 泛型数学宏

### 1.6 时间 (time.h, sys/time.h, sys/times.h)
- `time.c` — asctime, clock, clock_gettime, ctime, difftime, gmtime, localtime, mktime, strftime, strptime, time, tzset, 等
- `sys_time.c` — gettimeofday, settimeofday, 等
- `sys_times.c` — times

### 1.7 信号 (signal.h)
- `signal.c` — signal, raise, sigaction, sigaddset, sigdelset, sigemptyset, sigfillset, sigismember, sigpending, sigprocmask, sigsuspend, 等

### 1.8 进程控制 (unistd.h, spawn.h)
- `unistd.c` — _exit, access, alarm, chdir, chown, close, ctermid, dup, dup2, encrypt, execl, execle, execlp, execv, execve, execvp, fchdir, fchown, fork, fpathconf, fsync, ftruncate, getcwd, getegid, geteuid, getgid, getgroups, gethostid, gethostname, getlogin, getopt, getpgid, getpgrp, getpid, getppid, getsid, getuid, isatty, lseek, pathconf, pause, pipe, pread, pwrite, read, rmdir, setegid, seteuid, setgid, setpgid, setpgrp, setsid, setuid, sleep, swab, symlink, sync, sysconf, tcgetpgrp, tcsetpgrp, truncate, ttyname, unlink, write, 等
- `spawn.c` — posix_spawn 系列

### 1.9 文件操作 (fcntl.h, sys/stat.h, dirent.h, sys/mman.h, sys/uio.h, sys/statvfs.h)
- `fcntl.c` — open, creat, fcntl, 等
- `sys_stat.c` — stat, fstat, lstat, mkdir, mkfifo, chmod, fchmod, umask, 等
- `dirent.c` — opendir, readdir, rewinddir, closedir, dirfd, 等
- `sys_mman.c` — mmap, munmap, mprotect, msync, mlock, munlock, 等
- `sys_uio.c` — readv, writev
- `sys_statvfs.c` — statvfs, fstatvfs

### 1.10 用户/组 (pwd.h, grp.h)
- `pwd.c` — getpwnam, getpwuid, getpwent, setpwent, endpwent, 等
- `grp.c` — getgrnam, getgrgid, getgrent, setgrent, endgrent, 等

### 1.11 网络 (sys/socket.h, netdb.h, arpa/inet.h, net/if.h, netinet/in.h, netinet/tcp.h)
- `sys_socket.c` — socket, bind, connect, listen, accept, send, recv, sendto, recvfrom, getsockopt, setsockopt, 等
- `netdb.c` — getaddrinfo, freeaddrinfo, getnameinfo, gethostbyname, 等
- `arpa_inet.c` — inet_addr, inet_ntoa, inet_pton, inet_ntop, 等
- `net_if.c` — if_nametoindex, if_indextoname, 等
- `netinet_in.c` — IPPROTO_*, INET_* 常量
- `netinet_tcp.c` — TCP_* 常量

### 1.12 线程 (pthread.h)
- `pthread.c` — pthread_create, pthread_join, pthread_mutex_*, pthread_cond_*, pthread_rwlock_*, pthread_key_*, 等

### 1.13 其他 API 测试
- `aio.c` — 异步 IO
- `assert.c` — assert
- `cpio.c` — cpio 格式常量
- `dlfcn.c` — dlopen, dlsym, dlclose, dlerror
- `errno.c` — errno
- `fmtmsg.c` — fmtmsg
- `fnmatch.c` — fnmatch
- `ftw.c` — ftw, nftw
- `glob.c` — glob, globfree
- `iconv.c` — iconv_open, iconv, iconv_close
- `inttypes.c` — inttypes 宏
- `iso646.c` — iso646 宏
- `langinfo.c` — nl_langinfo, nl_langinfo_l
- `libgen.c` — basename, dirname
- `limits.c` — limits.h 常量
- `locale.c` — setlocale, localeconv, 等
- `monetary.c` — strfmon
- `mqueue.c` — mq_open, mq_close, mq_send, mq_receive, 等
- `ndbm.c` — dbm 操作
- `nl_types.c` — catopen, catgets, catclose
- `poll.c` — poll
- `regex.c` — regcomp, regexec, regfree, 等
- `sched.c` — 调度 API
- `search.c` — hsearch, hcreate, tsearch, tfind, lsearch, lfind, 等
- `semaphore.c` — sem_open, sem_close, sem_wait, sem_post, 等
- `setjmp.c` — setjmp, longjmp
- `stdarg.c` — va_list
- `stdbool.c` — bool 类型
- `stddef.c` — size_t, ptrdiff_t, NULL, offsetof
- `stdint.c` — stdint 类型
- `syslog.c` — syslog, openlog, closelog, setlogmask
- `sys_ipc.c` — ftok
- `sys_msg.c` — msgget, msgsnd, msgrcv, msgctl
- `sys_sem.c` — semget, semop, semctl
- `sys_shm.c` — shmget, shmat, shmdt, shmctl
- `sys_select.c` — select, pselect
- `sys_types.c` — 系统类型定义
- `sys_un.c` — AF_UNIX
- `sys_utsname.c` — uname
- `sys_wait.c` — wait, waitpid
- `tar.c` — tar 格式常量
- `termios.c` — 终端控制
- `utmpx.c` — utmpx 登录记录
- `wchar.c` — 宽字符函数
- `wctype.c` — 宽字符分类
- `wordexp.c` — wordexp

---

## 二、src/common/ — 公共工具库 (10 个文件)

这些文件编译为 `libtest.a`，为所有测试提供基础框架。**不是测试文件**。

| 文件 | 说明 |
|------|------|
| `test.h` | 测试框架头文件，定义 `t_error()`, `t_status` 等 |
| `runtest.c` | 测试运行器，超时后终止测试进程 |
| `mtest.c` / `mtest.h` | 数学测试支撑函数 |
| `options.h` / `options.h.in` | 编译选项检测 |
| `fdfill.c` | 文件描述符填充工具 |
| `memfill.c` | 内存填充工具 |
| `vmfill.c` | 虚拟内存填充工具 |
| `path.c` | 路径处理工具 |
| `print.c` | 打印工具 |
| `rand.c` | 随机数生成工具 |
| `setrlim.c` | 资源限制设置 |
| `utf8.c` | UTF-8 编码工具 |

---

## 三、src/functional/ — 功能测试 (54 个文件)

运行时功能测试，每个文件编译为独立的动态和静态链接可执行文件，旨在覆盖 libc 的主要功能。

### 3.1 字符串操作 (6 个文件)
- `string.c` — strcpy, strcat, strcmp, strchr, strrchr, strcspn, strspn, strpbrk, strstr, strtok, strtok_r, strerror, strlen, strnlen, memcpy, memmove, memset, memchr, memcmp, stpcpy, stpncpy
- `string_memcpy.c` — memcpy 专项测试
- `string_memmem.c` — memmem 专项测试
- `string_memset.c` — memset 专项测试
- `string_strchr.c` — strchr 专项测试
- `string_strcspn.c` — strcspn 专项测试
- `string_strstr.c` — strstr 专项测试

### 3.2 格式化输入输出 (5 个文件)
- `snprintf.c` — snprintf 完整测试（整数、浮点、字符串、宽度、精度等）
- `sscanf.c` — sscanf 测试
- `sscanf_long.c` — sscanf 长字符串测试
- `fscanf.c` — fscanf 测试
- `swprintf.c` — swprintf 宽字符格式化

### 3.3 数值转换 (6 个文件)
- `strtod.c` — strtod 通用测试
- `strtod_simple.c` — strtod 简单用例
- `strtod_long.c` — strtod 长字符串
- `strtof.c` — strtof 测试
- `strtold.c` — strtold 测试
- `strtol.c` — strtol, strtoul, strtoll, strtoull 测试

### 3.4 宽字符 (4 个文件)
- `wcsstr.c` — wcsstr 宽字符串搜索
- `wcstol.c` — wcstol, wcstoul, wcstoll 测试
- `fwscanf.c` — fwscanf 宽字符输入
- `mbc.c` — 多字节字符转换

### 3.5 时间 (3 个文件)
- `time.c` — gmtime, mktime, timegm, 时区处理
- `strftime.c` — strftime 格式化时间
- `strptime.c` — strptime 解析时间

### 3.6 搜索与排序 (6 个文件)
- `qsort.c` — qsort 排序测试
- `search_hsearch.c` — hsearch (哈希表搜索)
- `search_insque.c` — insque/remque (队列操作)
- `search_lsearch.c` — lsearch/lfind (线性搜索)
- `search_tsearch.c` — tsearch/tfind/tdelete (二叉树搜索)
- `random.c` — random, srandom, 随机数生成

### 3.7 线程 (5 个文件)
- `pthread_mutex.c` — pthread_mutex 互斥锁
- `pthread_mutex_pi.c` — 优先级继承互斥锁
- `pthread_cond.c` — pthread_cond 条件变量
- `pthread_tsd.c` — pthread_key 线程私有数据
- `pthread_robust.c` — 健壮的互斥锁
- `pthread_cancel.c` — 线程取消
- `pthread_cancel-points.c` — 线程取消点

### 3.8 进程与IPC (9 个文件)
- `ipc_msg.c` — System V 消息队列
- `ipc_sem.c` — System V 信号量
- `ipc_shm.c` — System V 共享内存
- `sem_init.c` — 未命名信号量
- `sem_open.c` — 命名信号量
- `spawn.c` — posix_spawn 进程创建
- `popen.c` — popen/pclose 管道
- `vfork.c` — vfork
- `udiv.c` — 整数除法

### 3.9 文件系统与IO (8 个文件)
- `stat.c` — stat, fstat, lstat, S_ISDIR, S_ISCHR
- `fcntl.c` — fcntl, open, 文件锁
- `fdopen.c` — fdopen
- `memstream.c` — open_memstream 内存流
- `ungetc.c` — ungetc 字符回退
- `utime.c` — utime, utimes 文件时间
- `fnmatch.c` — fnmatch 文件名匹配
- `iconv_open.c` — iconv 字符编码转换

### 3.10 网络 (2 个文件)
- `socket.c` — socket, bind, sendto, recvfrom, TCP/UDP
- `inet_pton.c` — inet_pton, inet_ntop 地址转换

### 3.11 其他 (9 个文件)
- `argv.c` — argv 参数传递
- `basename.c` — basename
- `dirname.c` — dirname
- `clocale_mbfuncs.c` — C locale 多字节函数
- `clock_gettime.c` — clock_gettime
- `crypt.c` — crypt, crypt_r 密码加密
- `dlopen.c` + `dlopen_dso.c` — 动态加载共享库
- `env.c` — 环境变量
- `setjmp.c` — setjmp/longjmp
- `tgmath.c` — 泛型数学宏
- `tls_align.c` + `tls_align_dso.c` + `tls_align_dlopen.c` — TLS 对齐
- `tls_init.c` + `tls_init_dso.c` + `tls_init_dlopen.c` — TLS 初始化
- `tls_local_exec.c` — TLS 本地执行模型

---

## 四、src/math/ — 数学函数测试 (191 个测试文件)

每个数学函数有独立的测试文件。测试数据来自：

| 数据目录 | 数量 | 来源 |
|----------|------|------|
| `sanity/` | 194 | 基本正确性测试 |
| `special/` | 191 | 特殊值测试 (NaN, INF, 0, 边界) |
| `ucb/` | 41 | UC Berkeley 测试向量 |
| `crlibm/` | 17 | 正确舍入数学库测试向量 |
| `gen/` | 工具 | 测试向量生成器 |

### 4.1 三角函数
- `sin.c`, `sinf.c`, `sinl.c` — 正弦
- `cos.c`, `cosf.c`, `cosl.c` — 余弦
- `tan.c`, `tanf.c`, `tanl.c` — 正切
- `sincos.c`, `sincosf.c`, `sincosl.c` — 同时计算正弦和余弦
- `asin.c`, `asinf.c`, `asinl.c` — 反正弦
- `acos.c`, `acosf.c`, `acosl.c` — 反余弦
- `atan.c`, `atanf.c`, `atanl.c` — 反正切
- `atan2.c`, `atan2f.c`, `atan2l.c` — 双参数反正切

### 4.2 双曲函数
- `sinh.c`, `sinhf.c`, `sinhl.c` — 双曲正弦
- `cosh.c`, `coshf.c`, `coshl.c` — 双曲余弦
- `tanh.c`, `tanhf.c`, `tanhl.c` — 双曲正切
- `asinh.c`, `asinhf.c`, `asinhl.c` — 反双曲正弦
- `acosh.c`, `acoshf.c`, `acoshl.c` — 反双曲余弦
- `atanh.c`, `atanhf.c`, `atanhl.c` — 反双曲正切

### 4.3 指数与对数
- `exp.c`, `expf.c`, `expl.c` — e^x
- `exp2.c`, `exp2f.c`, `exp2l.c` — 2^x
- `exp10.c`, `exp10f.c`, `exp10l.c` — 10^x
- `expm1.c`, `expm1f.c`, `expm1l.c` — e^x - 1
- `log.c`, `logf.c`, `logl.c` — 自然对数
- `log2.c`, `log2f.c`, `log2l.c` — 以 2 为底对数
- `log10.c`, `log10f.c`, `log10l.c` — 以 10 为底对数
- `log1p.c`, `log1pf.c`, `log1pl.c` — log(1+x)
- `logb.c`, `logbf.c`, `logbl.c` — 提取指数

### 4.4 幂与根
- `pow.c`, `powf.c`, `powl.c` — 幂
- `pow10.c`, `pow10f.c`, `pow10l.c` — 10^x
- `sqrt.c`, `sqrtf.c`, `sqrtl.c` — 平方根
- `cbrt.c`, `cbrtf.c`, `cbrtl.c` — 立方根
- `hypot.c`, `hypotf.c`, `hypotl.c` — 欧几里德距离

### 4.5 舍入与取整
- `ceil.c`, `ceilf.c`, `ceill.c` — 向上取整
- `floor.c`, `floorf.c`, `floorl.c` — 向下取整
- `trunc.c`, `truncf.c`, `truncl.c` — 向零取整
- `round.c`, `roundf.c`, `roundl.c` — 四舍五入
- `rint.c`, `rintf.c`, `rintl.c` — 向最近偶数取整
- `nearbyint.c`, `nearbyintf.c`, `nearbyintl.c` — 不触发 inexact 的 rint
- `lrint.c`, `lrintf.c`, `lrintl.c` — 取整为 long
- `llrint.c`, `llrintf.c`, `llrintl.c` — 取整为 long long
- `lround.c`, `lroundf.c`, `lroundl.c` — 四舍五入为 long
- `llround.c`, `llroundf.c`, `llroundl.c` — 四舍五入为 long long

### 4.6 浮点操作
- `fabs.c`, `fabsf.c`, `fabsl.c` — 绝对值
- `fmod.c`, `fmodf.c`, `fmodl.c` — 取模
- `remainder.c`, `remainderf.c`, `remainderl.c` — IEEE 余数
- `remquo.c`, `remquof.c`, `remquol.c` — 余数和商
- `copysign.c`, `copysignf.c`, `copysignl.c` — 复制符号
- `frexp.c`, `frexpf.c`, `frexpl.c` — 分解为尾数和指数
- `ldexp.c`, `ldexpf.c`, `ldexpl.c` — 乘以 2 的幂
- `modf.c`, `modff.c`, `modfl.c` — 分解整数和小数部分
- `scalbn.c`, `scalbnf.c`, `scalbnl.c` — 乘以 FLT_RADIX 的幂
- `scalbln.c`, `scalblnf.c`, `scalblnl.c` — 乘以 FLT_RADIX 的幂 (long)
- `ilogb.c`, `ilogbf.c`, `ilogbl.c` — 提取指数 (整数)
- `nextafter.c`, `nextafterf.c`, `nextafterl.c` — 下一个可表示值
- `nexttoward.c`, `nexttowardf.c`, `nexttowardl.c` — 下一个可表示值 (long double)
- `fma.c`, `fmaf.c`, `fmal.c` — 融合乘加
- `fdim.c`, `fdimf.c`, `fdiml.c` — 正差值
- `fmax.c`, `fmaxf.c`, `fmaxl.c` — 最大值
- `fmin.c`, `fminf.c`, `fminl.c` — 最小值

### 4.7 特殊函数
- `erf.c`, `erff.c`, `erfl.c` — 误差函数
- `erfc.c`, `erfcf.c`, `erfcl.c` — 互补误差函数
- `tgamma.c`, `tgammaf.c`, `tgammal.c` — Gamma 函数
- `lgamma.c`, `lgammaf.c`, `lgammal.c` — log Gamma 函数
- `lgamma_r.c`, `lgammaf_r.c`, `lgammal_r.c` — log Gamma (可重入)
- `j0.c`, `j0f.c` — 第一类 Bessel 函数 J0
- `j1.c`, `j1f.c` — 第一类 Bessel 函数 J1
- `jn.c`, `jnf.c` — 第一类 Bessel 函数 Jn
- `y0.c`, `y0f.c` — 第二类 Bessel 函数 Y0
- `y1.c`, `y1f.c` — 第二类 Bessel 函数 Y1
- `yn.c`, `ynf.c` — 第二类 Bessel 函数 Yn

### 4.8 其他
- `fenv.c` — 浮点环境 (feclearexcept, fetestexcept, 等)
- `fpclassify.c` — fpclassify 宏
- `isless.c` — 比较宏
- `drem.c`, `dremf.c` — BSD 余数 (已废弃)
- `scalb.c`, `scalbf.c` — BSD scalb (已废弃)

---

## 五、src/regression/ — 回归测试 (48 个文件)

针对特定 bug 修复的回归测试。每个文件注释中通常包含对应的 musl commit hash 或 bug 描述。

### 5.1 线程相关 (8 个文件)
- `pthread_atfork-errno-clobber.c` — atfork 处理函数不覆盖 errno
- `pthread_cancel-sem_wait.c` — sem_wait 被取消
- `pthread_cond-smasher.c` — 条件变量竞争
- `pthread_cond_wait-cancel_ignored.c` — 条件变量等待取消
- `pthread_condattr_setclock.c` — 条件变量时钟属性
- `pthread_create-oom.c` — 线程创建 OOM
- `pthread_exit-cancel.c` — 线程退出取消
- `pthread_exit-dtor.c` — 线程退出析构
- `pthread_once-deadlock.c` — pthread_once 死锁
- `pthread-robust-detach.c` — 健壮互斥锁分离
- `pthread_rwlock-ebusy.c` — 读写锁 EBUSY

### 5.2 格式化输出/输入 (8 个文件)
- `printf-1e9-oob.c` — printf 1e9 越界
- `printf-fmt-g-round.c` — %g 中点舍入到偶数
- `printf-fmt-g-zeros.c` — %g 零值处理
- `printf-fmt-n.c` — %n 格式
- `scanf-bytes-consumed.c` — scanf 字节消耗
- `scanf-match-literal-eof.c` — scanf 字面匹配 EOF
- `scanf-nullbyte-char.c` — scanf 空字节 %c
- `sscanf-eof.c` — sscanf EOF

### 5.3 字符串处理 (3 个文件)
- `memmem-oob.c` — memmem 越界
- `memmem-oob-read.c` — memmem 越界读取
- `strverscmp.c` — strverscmp 版本比较

### 5.4 宽字符 (3 个文件)
- `iswspace-null.c` — iswspace(NULL)
- `wcsncpy-read-overflow.c` — wcsncpy 读取溢出
- `wcsstr-false-negative.c` — wcsstr 假阴性

### 5.5 正则表达式 (5 个文件)
- `regex-backref-0.c` — 反向引用
- `regex-bracket-icase.c` — 括号忽略大小写
- `regexec-nosub.c` — REG_NOSUB
- `regex-ere-backref.c` — ERE 反向引用
- `regex-escaped-high-byte.c` — 转义高位字节
- `regex-negated-range.c` — 否定范围

### 5.6 内存分配 (3 个文件)
- `malloc-0.c` — malloc(0) 返回唯一指针
- `malloc-brk-fail.c` — malloc brk 失败
- `malloc-oom.c` — malloc OOM

### 5.7 网络 (3 个文件)
- `dn_expand-empty.c` — dn_expand 空输入
- `dn_expand-ptr-0.c` — dn_expand 指针
- `inet_ntop-v4mapped.c` — inet_ntop IPv4 映射
- `inet_pton-empty-last-field.c` — inet_pton 空最后字段

### 5.8 文件IO (5 个文件)
- `daemon-failure.c` — daemon 失败
- `fflush-exit.c` — fflush 退出
- `fgets-eof.c` — fgets EOF
- `fgetwc-buffering.c` — fgetwc 缓冲
- `flockfile-list.c` — flockfile 列表
- `ftello-unflushed-append.c` — ftello 未刷新追加
- `rewind-clear-error.c` — rewind 清除错误

### 5.9 环境/用户 (4 个文件)
- `execle-env.c` — execle 环境
- `getpwnam_r-crash.c` — getpwnam_r 崩溃
- `getpwnam_r-errno.c` — getpwnam_r errno
- `putenv-doublefree.c` — putenv 双释放
- `setenv-oom.c` — setenv OOM

### 5.10 其他 (5 个文件)
- `fpclassify-invalid-ld80.c` — fpclassify 无效 80 位
- `iconv-roundtrips.c` — iconv 往返转换
- `lrand48-signextend.c` — lrand48 符号扩展
- `lseek-large.c` — lseek 大偏移
- `mbsrtowcs-overflow.c` — mbsrtowcs 溢出
- `mkdtemp-failure.c` — mkdtemp 失败
- `mkstemp-failure.c` — mkstemp 失败
- `raise-race.c` — raise 竞争
- `rlimit-open-files.c` — RLIMIT_NOFILE
- `sem_close-unmap.c` — sem_close 取消映射
- `setvbuf-unget.c` — setvbuf unget
- `sigaltstack.c` — sigaltstack
- `sigprocmask-internal.c` — sigprocmask 内部
- `sigreturn.c` — sigreturn
- `statvfs.c` — statvfs
- `syscall-sign-extend.c` — syscall 符号扩展
- `tls_get_new-dtv.c` + `tls_get_new-dtv_dso.c` — TLS DTV
- `uselocale-0.c` — uselocale(0)

---

## 六、src/musl/ — musl 内部测试 (1 个文件)

- `pleval.c` — 测试 musl 内部 `__pleval` API (用于 dcngettext 的复数规则求值器)

---

## 测试文件统计

| 目录 | 文件数 | 说明 |
|------|--------|------|
| `src/api/` | 80 | 编译时 API 签名检查 |
| `src/common/` | 10 | 测试框架库 (非测试) |
| `src/functional/` | 54 | 运行时功能测试 |
| `src/math/` | 191 | 数学函数测试 (不含数据文件) |
| `src/regression/` | 48 | 回归测试 |
| `src/musl/` | 1 | musl 内部 API 测试 |
| **总计** | **384** | **(含 common 10 个)** |