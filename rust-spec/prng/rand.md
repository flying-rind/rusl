# rand/srand Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn rand() -> i32;

#[no_mangle]
pub unsafe extern "C" fn srand(seed: u32);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `rand()`: 无参数，依赖全局种子（static mut seed: u64）。
- `srand(seed)`: seed 为任意 u32 值。

**[Post-condition]:**
- `rand()`: 返回 [0, RAND_MAX] 的伪随机整数。
- `srand(seed)`: 设置全局种子 = seed - 1。

### 不变量

**[Invariant]:** 全局种子 `seed` (static mut u64) 在多线程下无锁保护，存在数据竞争。

### 意图

标准 C 库的简单伪随机数生成器。使用 64 位 LCG（乘数 6364136223846793005，加数 1），返回高 31 位。

### 系统算法

```
rand(): seed = 6364136223846793005 * seed + 1; return (seed >> 33) as i32
srand(s): seed = (s - 1) as u64
```