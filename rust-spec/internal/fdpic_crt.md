# fdpic_crt.rs 规约

> **复杂度等级**: Level 2（复杂逻辑 — 包含唯一的 `__fdpic_fixup` 函数实现，需要意图描述 + 系统算法）

---

## 依赖图

```
(外部) core::ffi           ──┐
crate::platform::types     ──┼──> fdpic_crt 模块
                              │
                              └── __fdpic_fixup (pub(crate) 函数)
```

本模块仅包含一个函数：`__fdpic_fixup`。它是 FDPIC ELF 程序 C 运行时启动的必要组件。FDPIC（Function Descriptor Position-Independent Code）用于无 MMU 的嵌入式 Linux 系统（如 ARM no-MMU、Blackfin、FR-V），其中共享库不能依赖虚拟内存映射来实现位置无关代码。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::ffi::c_void` | Rust core | 可用（`#![no_std]` 兼容） |
| `fdpic_loadseg` / `fdpic_loadmap` 结构体 | `crate::internal::dynlink` | 跨模块依赖 — 本模块内联定义了简化版结构体 |

---

## 符号规约

---

### `__fdpic_fixup`

```rust
// Rust 声明 (rusl)
#[cfg(feature = "fdpic")]
pub(crate) unsafe fn __fdpic_fixup(
    map: *const c_void,
    a: *mut usize,
    z: *const usize,
) -> *mut c_void;
```

```c
// C 等价声明 (musl)
hidden void *__fdpic_fixup(void *map, uintptr_t *a, uintptr_t *z)
```

[Visibility]: Internal — musl FDPIC CRT 内部函数，POSIX/C 标准未定义。仅在 `DL_FDPIC` 编译选项启用时被 C 运行时启动代码调用。

#### 功能意图 (Intent)

执行 FDPIC 程序的**函数描述符修复**（function descriptor fixup）。FDPIC 程序中，每个函数指针实际上是**函数描述符**（包含代码地址 + GOT 指针），而非直接的代码地址。`__fdpic_fixup` 在程序启动时将由内核传递的加载映射表（loadmap）信息用于修正这些函数描述符中的地址。

此函数被 C 运行时启动代码（`crt1.o` / `__crt_fdpic`）在调用 `main()` 之前调用，修复以下内容：
- `.got` 段中的 GOT 指针
- 函数描述符中的代码地址
- 仅对**主程序**（main executable）执行修复，不涉及共享库

#### 前置条件 (Preconditions)

- **PRE-1**: `map` 可能是 `null`（非 FDPIC 加载器）或指向一个 FDPIC 加载映射表。
- **PRE-2**: `a` 指向待修复地址数组的起始位置。
- **PRE-3**: `z` 指向待修复地址数组的结束位置（通常是 GOT[1] 之后的位置）。
- **PRE-4**: `a < z`（至少有一个条目需要处理）。
- **PRE-5**: 若 `map != null`，其内部结构必须符合 FDPIC 加载映射表格式：
  - `version` 和 `nsegs` 字段有效
  - `segs[]` 数组长度至少为 `nsegs`

#### 后置条件 (Postconditions)

- **Case 1 (`map == null`——非 FDPIC 加载器)**:
  - **POST-1**: 返回 `z.sub(1).read()` 的值作为 `*mut c_void`，即 GOT 指针（Global Offset Table 指针由非 FDPIC 加载器设置）。
  - **POST-2**: `*a` 到 `*(z-1)` 的内容不被修改。

- **Case 2 (`map != null`——FDPIC 加载器)**:
  - **POST-1**: 返回修正后的 GOT 指针。
  - **POST-2**: 数组 `[a, z)` 中的每个地址被修正：将原始的虚拟地址（p_vaddr）转换为实际的加载地址（addr）。
  - **POST-3**: 修正后的地址反映了内核实际加载该程序段的位置。

#### 系统算法 (System Algorithm)

```rust
pub(crate) unsafe fn __fdpic_fixup(map: *const c_void, a: *mut usize, z: *const usize) -> *mut c_void {
    // 特判：非 FDPIC 加载器
    if map.is_null() {
        return z.sub(1).read() as *mut c_void;
    }

    // 解析加载映射表的简化结构体
    // 结构体内联定义，避免外部依赖
    let lm = &*(map as *const FdpicLoadmap);
    let segs = core::slice::from_raw_parts(
        (map as *const u8).add(core::mem::size_of::<FdpicLoadmap>()) as *const FdpicLoadseg,
        lm.nsegs as usize,
    );

    let nsegs = lm.nsegs as usize;
    let mut rseg = 0usize;
    let mut vseg = 0usize;
    let mut a = a;

    loop {
        // 步骤 1: 定位当前待修复地址所属的"实际"加载段
        while *a - segs[rseg].p_vaddr >= segs[rseg].p_memsz {
            rseg += 1;
            if rseg == nsegs { rseg = 0; }
        }

        // 步骤 2: 计算修正后的地址
        let r = (*a + segs[rseg].addr - segs[rseg].p_vaddr) as *mut usize;

        // 步骤 3: 若没有更多条目，返回修正后的 GOT 指针
        a = a.add(1);
        if a as *const usize == z {
            return r as *mut c_void;
        }

        // 步骤 4: 定位目标地址所属的"虚拟"加载段
        while *r - segs[vseg].p_vaddr >= segs[vseg].p_memsz {
            vseg += 1;
            if vseg == nsegs { vseg = 0; }
        }

        // 步骤 5: 修正目标地址
        *r += segs[vseg].addr - segs[vseg].p_vaddr;
    }
}
```

**算法详细解释**:

该算法遍历一个地址数组 `[a, z)`，对每个地址执行**两次定位**：

1. **第一次定位（rseg）**: 找到地址 `*a` 属于哪个**实际加载段**——即内核将该段映射到的物理/实际位置。遍历 `segs[]`，寻找满足 `*a - p_vaddr < p_memsz` 的段。计算修正后的地址：`r = *a + (addr - p_vaddr)`，即将虚拟地址偏移应用于实际加载地址。

2. **第二次定位（vseg）**: 找到 `*r`（修正后地址指向的值）属于哪个**虚拟加载段**。这处理了跨段引用——当 GOT 条目或函数描述符指向的地址位于不同段时，需要再次应用偏移。

3. **修正**: `*r += addr - p_vaddr`，将 GOT/描述符中的虚拟地址转换为实际地址。

**Key Insight**: 算法在 `seg[]` 数组上线性搜索（而非二分搜索），因为典型的 FDPIC 程序只有少数几个段（通常 2-3 个），且段在虚拟地址空间中连续排列，顺序扫描足够高效。当搜索超出 `nsegs` 时回绕到 0（`rseg = 0`），处理了段数组不覆盖整个地址空间的情况。

#### 不变量 (Invariants)

- **INV-1 (终止性)**: 因为 `a` 每次递增，且 `a == z` 时返回，循环在有穷步骤内终止。
- **INV-2 (地址存在性)**: 对于任何有效的 `*a`，必存在某个 `segs[rseg]` 满足 `*a - p_vaddr < p_memsz`（即地址落在某个段的范围内），否则程序加载本身就是无效的。
- **INV-3 (返回值)**: 返回值始终是 `a` 数组中最后一个元素修正后的地址，即 GOT 指针。
- **INV-4 (非 FDPIC 兼容)**: 当 `map == null` 时，函数直接返回 `z.sub(1).read()`，不做任何修改。这使得同一段代码在 FDPIC 和非 FDPIC 系统上都能正常工作。

#### 边缘情况

| 场景 | 行为 |
|------|------|
| `map == null` | 直接返回 `z[-1]`，不修改数组 |
| `nsegs == 1` | 单段程序，rseg 和 vseg 始终为 0 |
| 地址跨段回绕 | `rseg`/`vseg` 回绕到 0，重新开始搜索 |
| `a + 1 == z`（只有一个条目） | 循环只执行一次，返回修正后的地址 |

---

## 内部数据结构

### FdpicLoadseg

```rust
// Rust 声明 (rusl) — 模块私有
struct FdpicLoadseg {
    addr: usize,
    p_vaddr: usize,
    p_memsz: usize,
}
```

[Visibility]: Internal — 仅在 `__fdpic_fixup` 内部使用，不透出模块。

### FdpicLoadmap

```rust
// Rust 声明 (rusl) — 模块私有
#[repr(C)]
struct FdpicLoadmap {
    version: u16,
    nsegs: u16,
    // segs: FdpicLoadseg 灵活数组成员，通过指针运算访问
}
```

[Visibility]: Internal — 仅在 `__fdpic_fixup` 内部使用，不透出模块。

---

## 全局不变量

- **GINV-1**: `__fdpic_fixup` 仅在 FDPIC 目标平台被调用（由 `#[cfg(feature = "fdpic")]` 编译选项控制）。非 FDPIC 平台上的 CRT 代码不应调用此函数。
- **GINV-2**: 函数描述符修复必须在任何函数调用（包括 `main()` 本身）之前完成。在 FDPIC 系统中，未修复的函数描述符指向无效地址，调用将导致段错误。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `FdpicLoadmap` 完整定义 | `crate::internal::dynlink` | 本模块内联定义了简化版 |
| CRT 调用者 | CRT 启动代码 | 启动代码在跳转到 `main()` 前调用 |

---

## Rust 实现注意事项 (`#![no_std]`)

1. **条件编译**: 通过 `#[cfg(feature = "fdpic")]` 控制，非 FDPIC 目标平台（如标准 x86_64）不需要编译此模块。
2. **unsafe 范围**: 整个函数标记为 `unsafe fn`，因为调用者必须保证 `map`、`a`、`z` 指针有效且符合 FDPIC 语义。内部不再嵌套 `unsafe` 块。
3. **结构体**: `FdpicLoadmap` 和 `FdpicLoadseg` 使用 `#[repr(C)]` 确保与内核传递的 FDPIC 加载映射表内存布局一致。
4. **灵活数组成员**: `segs[]` 通过 `core::slice::from_raw_parts` 手动计算偏移构造切片来访问，不使用 DST。
5. **core 依赖**: 仅使用 `core::ffi::c_void`、`core::mem::size_of`、`core::slice::from_raw_parts`，完全兼容 `#![no_std]`。