# strsignal

## 函数签名
```c
char *strsignal(int signum);
```

## 意图
返回信号编号 signum 对应的描述字符串。

## 前置条件
- 无（signum 可以为任意整数值）

## 后置条件
- 若 signum 为有效信号编号（1 到 _NSIG-1），返回值为该信号的描述字符串（如 "Interrupt"）
- 若 signum 为 0 或无效编号，返回值为 "Unknown signal"
- 返回值为静态字符串，不可被调用者修改
- 返回值可能受 locale 影响（通过 LCTRANS_CUR 翻译）

## 不变量
- 数据表 strings[] 和 map[] 的内容不变

## 算法
1. 通过 sigmap 宏将 signum 映射到内部编号（处理平台间信号编号差异）
2. 若映射后的编号超出有效范围，设为 0（"Unknown signal"）
3. 遍历以 '\0' 分隔的 strings 表，跳过前 signum 个字符串
4. 返回经过 locale 翻译后的字符串指针

/* Rely */
[RELY]
Predefined Structures/Functions:
  const char *__lctrans_cur(const char *);  // 依赖1: 获取当前 locale 下的翻译字符串，musl 内部函数

Predefined Macros:
  SIGHUP, SIGINT, SIGQUIT, SIGILL, SIGTRAP, SIGABRT,    // 依赖2: 标准信号编号宏，定义于 <signal.h>
  SIGBUS, SIGFPE, SIGKILL, SIGUSR1, SIGSEGV, SIGUSR2,
  SIGPIPE, SIGALRM, SIGTERM, SIGSTKFLT, SIGCHLD,
  SIGCONT, SIGSTOP, SIGTSTP, SIGTTIN, SIGTTOU, SIGURG,
  SIGXCPU, SIGXFSZ, SIGVTALRM, SIGPROF, SIGWINCH,
  SIGPOLL, SIGPWR, SIGSYS, SIGEMT
  _NSIG                                              // 依赖3: 信号总数宏，定义于 <signal.h>

[GUARANTEE]
Exported Interface:
  char *strsignal(int signum);  // 本模块保证对外提供的接口签名
