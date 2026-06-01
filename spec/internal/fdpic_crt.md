# fdpic_crt.h 规约

> **源文件**: `/home/mangp/桌面/OS/musl/src/internal/fdpic_crt.h`
> **复杂度等级**: Level 2（复杂逻辑 — 包含唯一的 `__fdpic_fixup` 内联函数实现，需要意图描述 + 系统算法）

---

## 依赖图

```
(外部) <stdint.h>    ──┐
(外部) <features.h>  ──┼──> fdpic_crt.h ──> 使用者（C 运行时启动代码 crt1.o）
                        │
                        └── __fdpic_fixup (static inline 函数)
```

本文件仅包含一个函数：`__fdpic_fixup`。它是 FDPIC ELF 程序 C 运行时启动的必要组件。FDPIC（Function Descriptor Position-Independent Code）用于无 MMU 的嵌入式 Linux 系统（如 ARM no-MMU、Blackfin、FR-V），其中共享库不能依赖虚拟内存映射来实现位置无关代码。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `<stdint.h>` | C 标准库 | 跳过 |
| `<features.h>` | musl 内部头文件 | **跨文件依赖** — 提供 `hidden` 可见性宏 |
| `fdpic_loadseg` / `fdpic_loadmap` 结构体 | `dynlink.h`（见 dynlink.h spec） | **跨文件依赖** — 本文件内联定义了一个**简化版本**的 `fdpic_loadmap` |

---

## 符号规约

---

### `__fdpic_fixup`

```c
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

- **PRE-1**: `map` 可能是 `NULL`（非 FDPIC 加载器）或指向一个 FDPIC 加载映射表。
- **PRE-2**: `a` 指向待修复地址数组的起始位置。
- **PRE-3**: `z` 指向待修复地址数组的结束位置（通常是 GOT[1] 之后的位置）。
- **PRE-4**: `a < z`（至少有一个条目需要处理）。
- **PRE-5**: 若 `map != NULL`，其内部结构必须符合 FDPIC 加载映射表格式：
  - `version` 和 `nsegs` 字段有效
  - `segs[]` 数组长度至少为 `nsegs`

#### 后置条件 (Postconditions)

- **Case 1 (`map == NULL`——非 FDPIC 加载器)**:
  - **POST-1**: 返回 `(void *)z[-1]`，即 GOT 指针（Global Offset Table 指针由非 FDPIC 加载器设置）。
  - **POST-2**: `*a` 到 `*(z-1)` 的内容不被修改。

- **Case 2 (`map != NULL`——FDPIC 加载器)**:
  - **POST-1**: 返回修正后的 GOT 指针。
  - **POST-2**: 数组 `[a, z)` 中的每个地址被修正：将原始的虚拟地址（p_vaddr）转换为实际的加载地址（addr）。
  - **POST-3**: 修正后的地址反映了内核实际加载该程序段的位置。

#### 系统算法 (System Algorithm)

```c
hidden void *__fdpic_fixup(void *map, uintptr_t *a, uintptr_t *z)
{
    // 特判：非 FDPIC 加载器
    if (!map) return (void *)z[-1];

    // 解析加载映射表的简化结构体
    struct {
        unsigned short version, nsegs;
        struct fdpic_loadseg {
            uintptr_t addr, p_vaddr, p_memsz;
        } segs[];
    } *lm = map;

    int nsegs = lm->nsegs, rseg = 0, vseg = 0;

    for (;;) {
        // 步骤 1: 定位当前待修复地址所属的"实际"加载段
        while (*a - lm->segs[rseg].p_vaddr >= lm->segs[rseg].p_memsz)
            if (++rseg == nsegs) rseg = 0;

        // 步骤 2: 计算修正后的地址
        uintptr_t *r = (uintptr_t *)
            (*a + lm->segs[rseg].addr - lm->segs[rseg].p_vaddr);

        // 步骤 3: 若没有更多条目，返回修正后的 GOT 指针
        if (++a == z) return r;

        // 步骤 4: 定位目标地址所属的"虚拟"加载段
        while (*r - lm->segs[vseg].p_vaddr >= lm->segs[vseg].p_memsz)
            if (++vseg == nsegs) vseg = 0;

        // 步骤 5: 修正目标地址
        *r += lm->segs[vseg].addr - lm->segs[vseg].p_vaddr;
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
- **INV-4 (非 FDPIC 兼容)**: 当 `map == NULL` 时，函数直接返回 `z[-1]`，不做任何修改。这使得同一段 CRT 代码在 FDPIC 和非 FDPIC 系统上都能正常工作。

#### 边缘情况

| 场景 | 行为 |
|------|------|
| `map == NULL` | 直接返回 `z[-1]`，不修改数组 |
| `nsegs == 1` | 单段程序，rseg 和 vseg 始终为 0 |
| 地址跨段回绕 | `rseg`/`vseg` 回绕到 0，重新开始搜索 |
| `a + 1 == z`（只有一个条目） | 循环只执行一次，返回修正后的地址 |

---

## 全局不变量

- **GINV-1**: `__fdpic_fixup` 仅在 FDPIC 目标平台被调用（由 `DL_FDPIC` 编译选项控制）。非 FDPIC 平台上的 CRT 代码不应调用此函数。
- **GINV-2**: 函数描述符修复必须在任何函数调用（包括 `main()` 本身）之前完成。在 FDPIC 系统中，未修复的函数描述符指向无效地址，调用将导致段错误。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `hidden` 宏 | `features.h` | 可见性控制 |
| `uintptr_t` | `<stdint.h>`（C 标准库） | 无符号整型，匹配指针大小 |
| `fdpic_loadmap` 完整定义 | `dynlink.h`（见 dynlink.h spec） | 本文件内联定义了简化版 |
| CRT 调用者 | `crt/crt1.c` 或架构特定 CRT 汇编 | 启动代码在跳转到 `main()` 前调用 |

---

## Rust 实现提示 (`#![no_std]`)

`__fdpic_fixup` 的 Rust 等价实现：

```rust
// 注意: 仅在 FDPIC 目标平台使用
// 使用 #[cfg(target_os = "linux")] + FDPIC 特性标志控制编译

use core::ffi::c_void;

struct FdpicLoadseg {
    addr: usize,
    p_vaddr: usize,
    p_memsz: usize,
}

struct FdpicLoadmap {
    version: u16,
    nsegs: u16,
    // segs: 灵活数组成员，需要手动解析
}

unsafe fn fdpic_fixup(map: *const c_void, a: *mut usize, z: *const usize) -> *mut c_void {
    if map.is_null() {
        return (*z.sub(1)) as *mut c_void;
    }

    let lm = &*(map as *const FdpicLoadmap);
    let segs = core::slice::from_raw_parts(
        (map as *const u8).add(core::mem::size_of::<FdpicLoadmap>()) as *const FdpicLoadseg,
        lm.nsegs as usize
    );

    let nsegs = lm.nsegs as usize;
    let mut rseg = 0usize;
    let mut vseg = 0usize;
    let mut a = a;

    loop {
        while *a - segs[rseg].p_vaddr >= segs[rseg].p_memsz {
            rseg += 1;
            if rseg == nsegs { rseg = 0; }
        }
        let r = (*a + segs[rseg].addr - segs[rseg].p_vaddr) as *mut usize;
        a = a.add(1);
        if a as *const usize == z {
            return r as *mut c_void;
        }
        while *r - segs[vseg].p_vaddr >= segs[vseg].p_memsz {
            vseg += 1;
            if vseg == nsegs { vseg = 0; }
        }
        *r += segs[vseg].addr - segs[vseg].p_vaddr;
    }
}
```

在 `rusl` 中，若目标平台不是无 MMU FDPIC 系统（如标准 x86_64 Linux），此代码不需要实现。可通过 `#[cfg(target_has_atomic = "ptr")]` 或自定义 `cfg(feature = "fdpic")` 条件编译。