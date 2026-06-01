# random/srandom/initstate/setstate Rust 接口

## 复杂度分级: Level 3

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn random() -> i64;

#[no_mangle]
pub unsafe extern "C" fn srandom(seed: u32);

#[no_mangle]
pub unsafe extern "C" fn initstate(seed: u32, state: *mut u8, n: usize) -> *mut u8;

#[no_mangle]
pub unsafe extern "C" fn setstate(state: *mut u8) -> *mut u8;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `random()/srandom()`: 同 rand/srand。
- `initstate(seed, state, n)`: state 为调用者提供的缓冲区（>= n 字节），n >= 8 决定状态表大小。
- `setstate(state)`: state 必须由先前的 initstate 初始化。

**[Post-condition]:**
- `random()`: 返回 [0, 2^31-1] 的非负 i64。
- `initstate()`: 返回旧状态指针，切换至新状态。n < 8 时返回 null。
- `setstate()`: 返回旧状态指针，切换至先前保存的状态。

### 不变量

**[Invariant]:** 使用自旋锁保护全局状态。内部 LFSR 延迟混洗算法。状态表大小由 n 决定：
n>=256 → deg=63, 128→31, 64→15, 32→7, 8→3。

### 意图

BSD random() 族的实现。使用滞后斐波那契生成器（LFSR），周期远大于 rand()。

### 系统算法

```
initstate: 根据 size 选择 deg 参数，分配状态表
srandom: 64位LCG展开种子填充状态表 + 预混洗
random: LFSR 混洗: state[i] += state[j]; 返回 state[i] >> 1
锁机制: 使用内部自旋锁 (static mut LOCK) 保护
```