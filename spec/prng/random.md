# random 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <stdlib.h>

long random(void);
void srandom(unsigned int seed);
char *initstate(unsigned int seed, char *state, size_t n);
char *setstate(char *state);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `random/srandom`: 同 rand/srand。
- `initstate`: `state` 为调用者提供的状态缓冲区（>= n 字节），`n` 决定状态表大小（>= 8）。
- `setstate`: `state` 必须由先前的 initstate 初始化。

**[Post-condition]:**
- `random()`: 返回 [0, 2^31-1] 的非负 long。
- `initstate()`: 返回旧状态指针，将生成器切换至新状态。
- `setstate()`: 返回旧状态指针，切换至先前保存的状态。n < 8 时 initstate 返回 NULL。

### 不变量

**[Invariant]:** 状态表不变量: `ptr` 始终指向 `state[0..deg-1]` 范围。
公开接口由 `__random_lock` 保护（`random`, `srandom`, `initstate`, `setstate`）。
内部接口 `__random` 等无锁，由调用者保证互斥。

### 意图

BSD `random()` 族的实现。使用状态表（LFSR 结构）的延迟混洗算法，周期显著大于 `rand()`。`initstate`/`setstate` 允许管理多个独立随机流。

### 系统算法

```
Phase 1 (initstate): 根据 n 选 deg: n>=256→63, 128→31, 64→15, 32→7, 8→3
Phase 2 (srandom): LCG 展开种子填充状态表 + 10*deg 轮预混洗
Phase 3 (random): 读 *ptr++，若 ptr 越界则全表 LFSR 混洗: state[i] = state[(i+sep)%deg] - state[i]
Phase 4 (setstate): 切换 ptr 和 state 指针至目标状态
锁机制: 公开接口 LOCK(__random_lock)，内部 __ 版本无锁
```

## 依赖关系

### 依赖的函数
- `lcg31()`: 内部 static 函数，小状态 LCG（用于 n==0 退化模式）
- `lcg64()`: 内部 static 函数，64 位 LCG（用于种子展开）
- `savestate()`: 内部 static 函数，保存当前状态到 state[-1]
- `loadstate()`: 内部 static 函数，从 state[-1] 恢复状态
- `__srandom()`: 内部 static 函数，无锁版种子初始化

### 依赖的数据结构
- `static uint32_t init[]`: 默认状态表（32 个随机种子值）
- `static int n, i, j`: LFSR 参数（表大小、前指针、后指针）
- `static uint32_t *x`: 状态表指针
- `static volatile int lock[1]`: 自旋锁
- `volatile int *const __random_lockptr`: 导出的锁指针（供 atfork 使用）

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `<stdint.h>`: uint32_t/uint64_t 类型
- `"lock.h"`: LOCK/UNLOCK 宏
- `"fork_impl.h"`: atfork 锁注册

### 被依赖
- 应用层直接调用
- `__random_lockptr` 被 fork 子系统引用以在 fork 时重置锁
