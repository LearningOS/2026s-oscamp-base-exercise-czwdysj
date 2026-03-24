//! # SV39 三级页表
//!
//! 本练习模拟 RISC-V SV39 三级页表的构造和地址翻译。
//! 注意，实际上的三级页表实现并非如本练习中使用 HashMap 模拟，本练习仅作为模拟帮助学习。
//! 你需要实现页表的创建、映射和地址翻译（页表遍历）。
//!
//! ## 知识点
//! - SV39：39 位虚拟地址，三级页表
//! - VPN 拆分：VPN[2] (9bit) | VPN[1] (9bit) | VPN[0] (9bit)
//! - 页表遍历（page table walk）逐级查找
//! - 大页（2MB superpage）映射
//!
//! ## SV39 虚拟地址布局
//! ```text
//! 38        30 29       21 20       12 11        0
//! ┌──────────┬───────────┬───────────┬───────────┐
//! │ VPN[2]   │  VPN[1]   │  VPN[0]   │  offset   │
//! │  9 bits  │  9 bits   │  9 bits   │  12 bits  │
//! └──────────┴───────────┴───────────┴───────────┘
//! ```

use std::collections::HashMap;

/// 页大小 4KB
pub const PAGE_SIZE: usize = 4096;
/// 每级页表有 512 个条目 (2^9)
pub const PT_ENTRIES: usize = 512;

/// PTE 标志位
pub const PTE_V: u64 = 1 << 0;
pub const PTE_R: u64 = 1 << 1;
pub const PTE_W: u64 = 1 << 2;
pub const PTE_X: u64 = 1 << 3;

/// PPN 在 PTE 中的偏移
const PPN_SHIFT: u32 = 10;

/// 页表节点：一个包含 512 个条目的数组
#[derive(Clone)]
pub struct PageTableNode {
    pub entries: [u64; PT_ENTRIES],
}

impl PageTableNode {
    pub fn new() -> Self {
        Self {
            entries: [0; PT_ENTRIES],
        }
    }
}

impl Default for PageTableNode {
    fn default() -> Self {
        Self::new()
    }
}

/// 模拟的三级页表。
///
/// 使用 HashMap<u64, PageTableNode> 模拟物理内存中的页表页。
/// `root_ppn` 是根页表所在的物理页号。
pub struct Sv39PageTable {
    /// 物理页号 -> 页表节点
    nodes: HashMap<u64, PageTableNode>,
    /// 根页表的物理页号
    pub root_ppn: u64,
    /// 下一个可分配的物理页号（简易分配器）
    next_ppn: u64,
}

/// 翻译结果
#[derive(Debug, PartialEq)]
pub enum TranslateResult {
    Ok(u64),
    PageFault,
}

impl Sv39PageTable {
    pub fn new() -> Self {
        let mut pt = Self {
            nodes: HashMap::new(),
            root_ppn: 0x80000,
            next_ppn: 0x80001,
        };
        pt.nodes.insert(pt.root_ppn, PageTableNode::new());
        pt
    }

    /// 分配一个新的物理页并初始化为空页表节点，返回其 PPN。
    fn alloc_node(&mut self) -> u64 {
        let ppn = self.next_ppn;
        self.next_ppn += 1;
        self.nodes.insert(ppn, PageTableNode::new());
        ppn
    }

    // ------------------------------------------------------------------------
    // 实验 1: 提取 VPN 索引 (extract_vpn)
    // 目标: 从虚拟地址中提取指定层级的 VPN 索引。
    // ------------------------------------------------------------------------

    /// 从 39 位虚拟地址中提取第 `level` 级的 VPN。
    /// Extract VPN of `level` from 39-bit virtual address.
    ///
    /// - level=2: 取 bits [38:30] / bits [38:30]
    /// - level=1: 取 bits [29:21] / bits [29:21]
    /// - level=0: 取 bits [20:12] / bits [20:12]
    ///
    /// 提示：右移 (12 + level * 9) 位，然后与 0x1FF 做掩码。
    /// Hint: Right shift by (12 + level * 9) bits, then mask with 0x1FF.
    pub fn extract_vpn(va: u64, level: usize) -> usize {
        // 计算每一级 VPN 的位移量：12 (页内偏移) + level * 9 (每级页表索引位)
        ((va >> (12 + level * 9)) & 0x1FF) as usize
    }

    // ------------------------------------------------------------------------
    // 实验 2: 建立 4KB 页映射 (map_page)
    // 目标: 实现三级页表的普通页映射。
    // ------------------------------------------------------------------------

    /// 建立从虚拟页到物理页的映射（4KB 页）。
    /// Map a virtual page to a physical page (4KB page).
    ///
    /// 参数 / Parameters：
    /// - `va`: 虚拟地址 / Virtual address (automatically aligned to page boundary)
    /// - `pa`: 物理地址 / Physical address (automatically aligned to page boundary)
    /// - `flags`: 标志位 / Flags (e.g., PTE_V | PTE_R | PTE_W)
    pub fn map_page(&mut self, va: u64, pa: u64, flags: u64) {
        let mut curr_ppn = self.root_ppn;

        // 遍历 level 2 和 level 1，确保中间页表节点存在
        for level in (1..=2).rev() {
            let vpn = Self::extract_vpn(va, level);

            // 先获取当前 PTE 的值，避免持续锁定 self.nodes
            let pte = self.nodes.get(&curr_ppn).expect("Node must exist").entries[vpn];

            if pte & PTE_V == 0 {
                // 如果当前页表项无效，分配一个新的页表节点
                let next_ppn = self.alloc_node();
                // 重新获取可变引用并更新
                let node = self.nodes.get_mut(&curr_ppn).expect("Node must exist");
                node.entries[vpn] = (next_ppn << PPN_SHIFT) | PTE_V;
                curr_ppn = next_ppn;
            } else {
                // 提取下一级页表的 PPN
                curr_ppn = pte >> PPN_SHIFT;
            }
        }

        // 在 level 0 写入最终的叶子节点映射
        let vpn0 = Self::extract_vpn(va, 0);
        let node0 = self
            .nodes
            .get_mut(&curr_ppn)
            .expect("Level 0 node must exist");
        node0.entries[vpn0] = ((pa >> 12) << PPN_SHIFT) | flags | PTE_V;
    }

    // ------------------------------------------------------------------------
    // 实验 3: 三级页表遍历翻译 (translate)
    // 目标: 模拟硬件 MMU 的地址翻译过程。
    // ------------------------------------------------------------------------

    /// 遍历三级页表，将虚拟地址翻译为物理地址。
    /// Walk the 3-level page table to translate virtual address to physical address.
    ///
    /// 步骤 / Steps：
    /// 1. 从根页表（root_ppn）开始 / Start from root_ppn.
    /// 2. 对每一级（2, 1, 0）： / For each level (2, 1, 0):
    ///    a. 用 VPN[level] 索引当前页表节点 / Index node with VPN[level].
    ///    b. 如果 PTE 无效（!PTE_V），返回 PageFault / Return PageFault if PTE is invalid.
    ///    c. 如果 PTE 是叶节点（R|W|X 有任一置位），提取 PPN 计算物理地址 / If leaf PTE, calculate PA.
    ///    d. 否则用 PTE 中的 PPN 进入下一级页表 / Else move to next level using PPN.
    /// 3. level 0 的 PTE 必须是叶节点 / Level 0 PTE must be a leaf.
    pub fn translate(&self, va: u64) -> TranslateResult {
        let mut curr_ppn = self.root_ppn;

        for level in (0..=2).rev() {
            let vpn = Self::extract_vpn(va, level);
            let node = match self.nodes.get(&curr_ppn) {
                Some(n) => n,
                None => return TranslateResult::PageFault,
            };

            let pte = node.entries[vpn];
            if pte & PTE_V == 0 {
                return TranslateResult::PageFault;
            }

            // 检查 R/W/X 位，判断是否为叶子节点
            if pte & (PTE_R | PTE_W | PTE_X) != 0 {
                // 找到了叶子节点（可能是 4KB, 2MB 或 1GB 大页）
                let ppn = pte >> PPN_SHIFT;
                // 计算页内偏移掩码：
                // level 0 (4KB) -> 低 12 位
                // level 1 (2MB) -> 低 21 位
                // level 2 (1GB) -> 低 30 位
                let offset_mask = (1 << (12 + level * 9)) - 1;
                let pa = (ppn << 12) | (va & offset_mask);
                return TranslateResult::Ok(pa);
            }

            // 不是叶子节点，进入下一级
            if level == 0 {
                // 如果到了第 0 级还不是叶子，说明无效
                return TranslateResult::PageFault;
            }
            curr_ppn = pte >> PPN_SHIFT;
        }

        TranslateResult::PageFault
    }

    // ------------------------------------------------------------------------
    // 实验 4: 建立 2MB 大页映射 (map_superpage)
    // 目标: 实现三级页表中的大页映射。
    // ------------------------------------------------------------------------

    /// 建立大页映射（2MB superpage，在 level 1 设叶子 PTE）。
    /// Map a 2MB superpage (set leaf PTE at level 1).
    ///
    /// 2MB = 512 × 4KB，对齐要求：va 和 pa 都必须 2MB 对齐。
    /// Alignment: va and pa must be 2MB-aligned.
    ///
    /// 与 map_page 类似，但只遍历到 level 1 就写入叶子 PTE。
    /// Similar to map_page, but write leaf PTE at level 1.
    pub fn map_superpage(&mut self, va: u64, pa: u64, flags: u64) {
        let mega_size: u64 = (PAGE_SIZE * PT_ENTRIES) as u64; // 2MB
        assert_eq!(va % mega_size, 0, "va must be 2MB-aligned");
        assert_eq!(pa % mega_size, 0, "pa must be 2MB-aligned");

        // 1. 遍历 level 2，确保中间页表节点存在
        let vpn2 = Self::extract_vpn(va, 2);

        // 先获取根页表中的 PTE 值
        let pte2 = self
            .nodes
            .get(&self.root_ppn)
            .expect("Root node must exist")
            .entries[vpn2];

        let curr_ppn = if pte2 & PTE_V == 0 {
            // 分配新节点
            let next_ppn = self.alloc_node();
            // 重新获取根页表并更新 PTE
            let root_node = self
                .nodes
                .get_mut(&self.root_ppn)
                .expect("Root node must exist");
            root_node.entries[vpn2] = (next_ppn << PPN_SHIFT) | PTE_V;
            next_ppn
        } else {
            pte2 >> PPN_SHIFT
        };

        // 2. 在 level 1 建立大页映射（叶子节点）
        let vpn1 = Self::extract_vpn(va, 1);
        let node1 = self
            .nodes
            .get_mut(&curr_ppn)
            .expect("Level 1 node must exist");

        // 设置标志位，由于设置了 R/W/X，它将成为一个 2MB 的大页叶子节点
        node1.entries[vpn1] = ((pa >> 12) << PPN_SHIFT) | flags | PTE_V;
    }
}

impl Default for Sv39PageTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vpn() {
        // VA = 0x0000_003F_FFFF_F000 (最大的 39 位地址的页边界)
        // VPN[2] = 0xFF (bits 38:30)
        // VPN[1] = 0x1FF (bits 29:21)
        // VPN[0] = 0x1FF (bits 20:12)
        let va: u64 = 0x7FFFFFF000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 0x1FF);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0x1FF);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 0x1FF);
    }

    #[test]
    fn test_extract_vpn_simple() {
        // VA = 0x00000000 + page 1 = 0x1000
        // VPN[2] = 0, VPN[1] = 0, VPN[0] = 1
        let va: u64 = 0x1000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 1);
    }

    #[test]
    fn test_extract_vpn_level2() {
        // VPN[2] = 1 means bit 30 set -> VA >= 0x40000000
        let va: u64 = 0x40000000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 1);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 0);
    }

    #[test]
    fn test_map_and_translate_single() {
        let mut pt = Sv39PageTable::new();
        // 映射：VA 0x1000 -> PA 0x80001000
        pt.map_page(0x1000, 0x80001000, PTE_V | PTE_R);

        let result = pt.translate(0x1000);
        assert_eq!(result, TranslateResult::Ok(0x80001000));
    }

    #[test]
    fn test_translate_with_offset() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x2000, 0x90000000, PTE_V | PTE_R | PTE_W);

        // 访问 VA 0x2ABC -> PA 应为 0x90000ABC
        let result = pt.translate(0x2ABC);
        assert_eq!(result, TranslateResult::Ok(0x90000ABC));
    }

    #[test]
    fn test_translate_page_fault() {
        let pt = Sv39PageTable::new();
        assert_eq!(pt.translate(0x1000), TranslateResult::PageFault);
    }

    #[test]
    fn test_multiple_mappings() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x0000_1000, 0x8000_1000, PTE_V | PTE_R);
        pt.map_page(0x0000_2000, 0x8000_5000, PTE_V | PTE_R | PTE_W);
        pt.map_page(0x0040_0000, 0x9000_0000, PTE_V | PTE_R);

        assert_eq!(pt.translate(0x1234), TranslateResult::Ok(0x80001234));
        assert_eq!(pt.translate(0x2000), TranslateResult::Ok(0x80005000));
        assert_eq!(pt.translate(0x400100), TranslateResult::Ok(0x90000100));
    }

    #[test]
    fn test_map_overwrite() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x1000, 0x80001000, PTE_V | PTE_R);
        assert_eq!(pt.translate(0x1000), TranslateResult::Ok(0x80001000));

        pt.map_page(0x1000, 0x90002000, PTE_V | PTE_R);
        assert_eq!(pt.translate(0x1000), TranslateResult::Ok(0x90002000));
    }

    #[test]
    fn test_superpage_mapping() {
        let mut pt = Sv39PageTable::new();
        // 2MB 大页映射：VA 0x200000 -> PA 0x80200000
        pt.map_superpage(0x200000, 0x80200000, PTE_V | PTE_R | PTE_W);

        // 大页内不同偏移都应命中
        assert_eq!(pt.translate(0x200000), TranslateResult::Ok(0x80200000));
        assert_eq!(pt.translate(0x200ABC), TranslateResult::Ok(0x80200ABC));
        assert_eq!(pt.translate(0x2FF000), TranslateResult::Ok(0x802FF000));
    }

    #[test]
    fn test_superpage_and_normal_coexist() {
        let mut pt = Sv39PageTable::new();
        // 大页映射在第一个 2MB 区域
        pt.map_superpage(0x0, 0x80000000, PTE_V | PTE_R);
        // 普通页在不同的 VPN[2] 区域
        pt.map_page(0x40000000, 0x90001000, PTE_V | PTE_R);

        assert_eq!(pt.translate(0x100), TranslateResult::Ok(0x80000100));
        assert_eq!(pt.translate(0x40000000), TranslateResult::Ok(0x90001000));
    }
}
