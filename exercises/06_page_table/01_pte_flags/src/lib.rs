//! # Page Table Entry Flags
//! # 页表项标志位 (PTE Flags)
//!
//! In this exercise, you will learn the structure of RISC-V SV39 Page Table Entry (PTE),
//! and construct/parse page table entries through bit operations.
//! 在本练习中，你将学习 RISC-V SV39 页表项 (PTE) 的结构，并通过位操作构造和解析页表项。
//!
//! ## Concepts
//! ## 核心概念
//! - RISC-V SV39 page table entry 64-bit layout / RISC-V SV39 页表项 64 位布局
//! - Bit operations to construct/extract fields / 使用位操作构造/提取字段
//! - Meaning of PTE flags (V/R/W/X/U/G/A/D) / PTE 标志位的含义 (V/R/W/X/U/G/A/D)
//!
//! ## SV39 PTE Layout (64-bit)
//! ## SV39 PTE 布局 (64位)
//! ```text
//! 63    54 53        10 9  8 7 6 5 4 3 2 1 0
//! ┌───────┬────────────┬────┬─┬─┬─┬─┬─┬─┬─┬─┐
//! │ Rsvd  │  PPN[2:0]  │ RSW│D│A│G│U│X│W│R│V│
//! │ 10bit │  44 bits   │ 2b │ │ │ │ │ │ │ │ │
//! └───────┴────────────┴────┴─┴─┴─┴─┴─┴─┴─┴─┘
//! ```
//! - V (Valid): Valid bit indicating whether the page table entry is valid.
//!   V (有效位)：指示该页表项是否有效。
//!
//! - R/W/X (Read/Write/Execute): Permission bits for read, write, and execute access respectively.
//!   R/W/X (读/写/执行)：分别表示读、写和执行权限。
//!
//! - U (User): User-accessible bit, allowing access from user mode.
//!   U (用户)：用户访问位，允许在用户模式下访问。
//!
//! - G (Global): Global mapping bit (typically used for kernel space to avoid TLB flushes).
//!   G (全局)：全局映射位（通常用于内核空间以避免 TLB 刷新）。
//!
//! - A (Accessed): Accessed bit, set by hardware when the page is accessed.
//!   A (已访问)：已访问位，当页面被访问时由硬件设置。
//!
//! - D (Dirty): Dirty bit, set by hardware when the page is written to.
//!   D (脏位)：脏位，当页面被写入时由硬件设置。
//!
//! - RSW (Reserved for Supervisor Software): Two bits reserved for operating system software use.
//!   RSW (为内核软件预留)：预留给操作系统软件使用的两位。
//!
//! - PPN (Physical Page Number): Physical page number occupying 44 bits (bits [53:10]), specifying the base address of the physical page frame.
//!   PPN (物理页号)：占用 44 位 (第 53:10 位)，指定物理页帧的基地址。
//!
//! - PPN[2:0] (Physical Page Number): In the RISC-V SV39 paging mechanism, the Physical Page Number (PPN) is divided into three parts, which are referred to as PPN[2], PPN[1], and PPN[0]. This division is designed to support the indexing of multi-level page tables.
//!   PPN[2:0] (物理页号分段)：在 RISC-V SV39 分页机制中，PPN 被分为三部分：PPN[2], PPN[1], PPN[0]。这种划分是为了支持多级页表的索引。
//!
//! - Rsvd (Reserved): Reserved bits, typically set to 0.
//!   Rsvd (预留)：预留位，通常设置为 0。

/// PTE flag constants
/// PTE 标志位常量
pub const PTE_V: u64 = 1 << 0; // Valid / 有效位
pub const PTE_R: u64 = 1 << 1; // Readable / 可读
pub const PTE_W: u64 = 1 << 2; // Writable / 可写
pub const PTE_X: u64 = 1 << 3; // Executable / 可执行
pub const PTE_U: u64 = 1 << 4; // User accessible / 用户可访问
pub const PTE_G: u64 = 1 << 5; // Global / 全局
pub const PTE_A: u64 = 1 << 6; // Accessed / 已访问
pub const PTE_D: u64 = 1 << 7; // Dirty / 脏位

/// PPN field offset and mask in PTE
/// PTE 中 PPN 字段的偏移量和掩码
const PPN_SHIFT: u32 = 10;
const PPN_MASK: u64 = (1u64 << 44) - 1; // 44-bit PPN / 44 位 PPN

// ------------------------------------------------------------------------
// 实验 1: 构造页表项 (make_pte)
// 目标: 将物理页号和标志位组合成一个 64 位的页表项。
// ------------------------------------------------------------------------

/// Construct a page table entry from physical page number (PPN) and flags.
/// 从物理页号 (PPN) 和标志位构造页表项。
///
/// PPN occupies bits [53:10], flags occupy bits [7:0].
/// PPN 占用第 [53:10] 位，标志位占用第 [7:0] 位。
///
/// Example: ppn=0x12345, flags=PTE_V|PTE_R|PTE_W
/// Result should be: (0x12345 << 10) | 0b111 = 0x48D14007
///
/// Hint: Shift PPN left by PPN_SHIFT bits, then OR with flags.
/// 提示：将 PPN 左移 PPN_SHIFT 位，然后与标志位进行按位或操作。
pub fn make_pte(ppn: u64, flags: u64) -> u64 {
    (ppn << PPN_SHIFT) | flags
}

// ------------------------------------------------------------------------
// 实验 2: 提取物理页号 (extract_ppn)
// 目标: 从 64 位的页表项中解析出 44 位的物理页号。
// ------------------------------------------------------------------------

/// Extract physical page number (PPN) from page table entry.
/// 从页表项中提取物理页号 (PPN)。
///
/// Hint: Right shift by PPN_SHIFT bits, then AND with PPN_MASK.
/// 提示：右移 PPN_SHIFT 位，然后与 PPN_MASK 进行按位与操作。
pub fn extract_ppn(pte: u64) -> u64 {
    (pte >> PPN_SHIFT) & PPN_MASK
}

// ------------------------------------------------------------------------
// 实验 3: 提取标志位 (extract_flags)
// 目标: 从页表项中解析出低 8 位的标志位。
// ------------------------------------------------------------------------

/// Extract flags (lower 8 bits) from page table entry.
/// 从页表项中提取标志位（低 8 位）。
pub fn extract_flags(pte: u64) -> u64 {
    pte & 0xff
}

// ------------------------------------------------------------------------
// 实验 4: 检查有效位 (is_valid)
// 目标: 判断页表项的 V 位是否被设置。
// ------------------------------------------------------------------------

/// Check whether page table entry is valid (V bit set).
/// 检查页表项是否有效（V 位被设置）。
pub fn is_valid(pte: u64) -> bool {
    // TODO: Check PTE_V
    // 任务：检查 PTE_V
    pte & PTE_V != 0
}

// ------------------------------------------------------------------------
// 实验 5: 判断是否为叶子节点 (is_leaf)
// 目标: 根据 R/W/X 位判断该 PTE 是指向物理页还是下一级页表。
// ------------------------------------------------------------------------

/// Determine whether page table entry is a leaf PTE.
/// 判断页表项是否为叶子页表项。
///
/// In SV39, if any of R, W, X bits is set, the PTE is a leaf,
/// pointing to the final physical page. Otherwise it points to next-level page table.
/// 在 SV39 中，如果 R、W、X 位中有任何一位被设置，则该 PTE 是叶子节点，
/// 指向最终的物理页。否则它指向下一级页表。
pub fn is_leaf(pte: u64) -> bool {
    // TODO: Check if any of R/W/X bits is set
    // 任务：检查是否设置了 R/W/X 位中的任何一个
    (pte & (PTE_R | PTE_W | PTE_X)) != 0
}

// ------------------------------------------------------------------------
// 实验 6: 权限检查 (check_permission)
// 目标: 验证 PTE 是否满足请求的读/写/执行权限要求。
// ------------------------------------------------------------------------

/// Check whether page table entry permits the requested access based on given permissions.
/// 根据给定的权限要求，检查页表项是否允许所请求的访问。
///
/// - `read`: requires read permission / 需要读权限
/// - `write`: requires write permission / 需要写权限
/// - `execute`: requires execute permission / 需要执行权限
///
/// Returns true iff: PTE is valid, and each requested permission is satisfied.
/// 返回 true 当且仅当：PTE 有效，且满足每个请求的权限。
pub fn check_permission(pte: u64, read: bool, write: bool, execute: bool) -> bool {
    // 任务：首先检查是否有效，然后检查每个请求的权限
    if !is_valid(pte) {
        return false;
    }
    // The logic is: if a permission is requested, the corresponding bit must be set.
    // If not requested, we don't care.
    // 逻辑是：如果请求了某个权限，那么相应的位必须被设置。
    // 如果没有请求，我们不关心该位。
    let r_ok = !read || (pte & PTE_R != 0);
    let w_ok = !write || (pte & PTE_W != 0);
    let x_ok = !execute || (pte & PTE_X != 0);
    r_ok && w_ok && x_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_pte_basic() {
        let pte = make_pte(0x12345, PTE_V | PTE_R | PTE_W);
        assert_eq!(extract_ppn(pte), 0x12345);
        assert_eq!(extract_flags(pte), PTE_V | PTE_R | PTE_W);
    }

    #[test]
    fn test_make_pte_zero() {
        let pte = make_pte(0, 0);
        assert_eq!(pte, 0);
        assert_eq!(extract_ppn(pte), 0);
        assert_eq!(extract_flags(pte), 0);
    }

    #[test]
    fn test_make_pte_all_flags() {
        let all = PTE_V | PTE_R | PTE_W | PTE_X | PTE_U | PTE_G | PTE_A | PTE_D;
        let pte = make_pte(0xABC, all);
        assert_eq!(extract_ppn(pte), 0xABC);
        assert_eq!(extract_flags(pte), all);
    }

    #[test]
    fn test_make_pte_large_ppn() {
        let ppn = (1u64 << 44) - 1; // maximum PPN
        let pte = make_pte(ppn, PTE_V);
        assert_eq!(extract_ppn(pte), ppn);
    }

    #[test]
    fn test_is_valid() {
        assert!(is_valid(make_pte(1, PTE_V)));
        assert!(!is_valid(make_pte(1, PTE_R))); // R set but V not set
        assert!(!is_valid(0));
    }

    #[test]
    fn test_is_leaf() {
        assert!(is_leaf(make_pte(1, PTE_V | PTE_R)));
        assert!(is_leaf(make_pte(1, PTE_V | PTE_X)));
        assert!(is_leaf(make_pte(1, PTE_V | PTE_R | PTE_W | PTE_X)));
        // Non-leaf: only V set, R/W/X all cleared
        assert!(!is_leaf(make_pte(1, PTE_V)));
        assert!(!is_leaf(make_pte(1, PTE_V | PTE_A | PTE_D)));
    }

    #[test]
    fn test_check_permission_read() {
        let pte = make_pte(1, PTE_V | PTE_R);
        assert!(check_permission(pte, true, false, false));
        assert!(!check_permission(pte, false, true, false));
        assert!(!check_permission(pte, false, false, true));
    }

    #[test]
    fn test_check_permission_rw() {
        let pte = make_pte(1, PTE_V | PTE_R | PTE_W);
        assert!(check_permission(pte, true, true, false));
        assert!(!check_permission(pte, true, true, true));
    }

    #[test]
    fn test_check_permission_all() {
        let pte = make_pte(1, PTE_V | PTE_R | PTE_W | PTE_X);
        assert!(check_permission(pte, true, true, true));
        assert!(check_permission(pte, true, false, false));
        assert!(check_permission(pte, false, false, false)); // no requirement = OK
    }

    #[test]
    fn test_check_permission_invalid() {
        // V not set, should return false even if R/W/X flags present
        let pte = make_pte(1, PTE_R | PTE_W | PTE_X);
        assert!(!check_permission(pte, true, false, false));
    }
}
