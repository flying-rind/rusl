# hsearch/hcreate/hdestroy Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn hcreate(nel: usize) -> i32;

#[no_mangle]
pub unsafe extern "C" fn hdestroy();

#[no_mangle]
pub unsafe extern "C" fn hsearch(item: ENTRY, action: ACTION) -> *mut ENTRY;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `hcreate(nel)`: nel 为预估哈希表容量。
- `hsearch(item, action)`: item.key 非 null；action 为 FIND(=0) 或 ENTER(=1)。
- 必须先 hcreate 再 hsearch，最后 hdestroy。

**[Post-condition]:**
- `hcreate(nel)`: 成功返回非零，失败返回 0。
- `hsearch(item, FIND)`: 找到返回 ENTRY 指针，未找到返回 null。
- `hsearch(item, ENTER)`: 找到返回已有 ENTRY 指针，否则插入并返回新 ENTRY 指针；内存不足返回 null。
- `hdestroy()`: 释放所有内部资源。

### 不变量

**[Invariant]:** 全局单例哈希表，非线程安全。使用开放寻址 + 二次探测。

### 意图

POSIX 标准哈希表管理。**此接口已过时**，推荐使用 hcreate_r 系列。

### 系统算法

```
hcreate(nel): calloc buckets + resize 到 2^n >= nel
hsearch(item, act): keyhash(key) -> 二次探测查找 -> 找到返回 / 未找到根据 action 决定
hdestroy(): free entries + free __tab
```