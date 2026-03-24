//! # Single‑Level Page Table Address Translation
//! # 单级页表地址翻译
//!
//! This exercise simulates a simple single‑level page table to help you understand the process of virtual‑to‑physical address translation.
//! 本练习模拟了一个简单的单级页表，以帮助你理解虚拟地址到物理地址的翻译过程。
//!
//! ## Concepts
//! ## 核心概念
//! - Virtual address = Virtual Page Number (VPN) + Page Offset (offset)
//!   虚拟地址 = 虚拟页号 (VPN) + 页内偏移 (offset)
//! - Page table: VPN → PPN mapping table
//!   页表：VPN 到 PPN（物理页号）的映射表
//! - Address translation: Physical address = PPN × PAGE_SIZE + offset
//!   地址翻译：物理地址 = PPN × 页大小 + 偏移量
//! - Page fault: accessing an unmapped virtual page
//!   缺页异常 (Page fault)：访问未映射的虚拟页
//!
//! ## Address Format (Simplified Model)
//! ## 地址格式 (简化模型)
//! ```text
//! Virtual address (32‑bit):
//! 31          12 11          0
//! ┌──────────────┬────────────┐
//! │   VPN (20 bits)  │ offset (12 bits) │
//! └──────────────┴────────────┘
//!
//! Page size: 4KB (2^12 = 4096 bytes)
//! ```

/// 页大小 4KB / Page size 4KB
pub const PAGE_SIZE: usize = 4096;
/// 页内偏移位数 / Number of bits for page offset
pub const PAGE_OFFSET_BITS: u32 = 12;

/// 页表项标志 / Page Table Entry flags
pub const PTE_VALID: u8 = 1 << 0;
pub const PTE_READ: u8 = 1 << 1;
pub const PTE_WRITE: u8 = 1 << 2;

/// 页表项 / Page Table Entry
#[derive(Clone, Copy, Debug)]
pub struct PageTableEntry {
    pub ppn: u32,
    pub flags: u8,
}

/// 翻译结果 / Translation Result
#[derive(Debug, PartialEq)]
pub enum TranslateResult {
    /// 翻译成功，得到物理地址 / Translation successful, returns physical address
    Ok(u32),
    /// 缺页：虚拟页未映射 / Page fault: virtual page is not mapped
    PageFault,
    /// 权限错误：尝试写入只读页 / Permission denied: attempt to write to a read-only page
    PermissionDenied,
}

/// 单级页表，最多支持 `MAX_PAGES` 个虚拟页。
/// Single-level page table, supporting up to `MAX_PAGES` virtual pages.
pub struct SingleLevelPageTable {
    entries: Vec<Option<PageTableEntry>>,
}

impl SingleLevelPageTable {
    /// 创建一个空页表，支持 `max_pages` 个虚拟页。
    /// Create an empty page table supporting `max_pages` virtual pages.
    pub fn new(max_pages: usize) -> Self {
        Self {
            entries: vec![None; max_pages],
        }
    }

    // ------------------------------------------------------------------------
    // 实验 1: 建立映射 (map)
    // 目标: 在页表中记录虚拟页号到物理页号的映射关系。
    // ------------------------------------------------------------------------

    /// 将虚拟页号 `vpn` 映射到物理页号 `ppn`，并设置标志位 `flags`。
    /// Map virtual page number `vpn` to physical page number `ppn` with `flags`.
    ///
    /// 提示：在 `entries[vpn]` 处存放一个 `PageTableEntry`。
    /// Hint: Store a `PageTableEntry` at `entries[vpn]`.
    pub fn map(&mut self, vpn: usize, ppn: u32, flags: u8) {
        // TODO: 在页表中建立 vpn -> ppn 的映射
        // 任务：将 Some(PageTableEntry { ppn, flags }) 赋值给 self.entries[vpn]
        self.entries[vpn] = Some(PageTableEntry { ppn, flags });
    }

    // ------------------------------------------------------------------------
    // 实验 2: 取消映射 (unmap)
    // 目标: 从页表中移除指定的虚拟页映射。
    // ------------------------------------------------------------------------

    /// 取消虚拟页号 `vpn` 的映射。
    /// Unmap the virtual page number `vpn`.
    pub fn unmap(&mut self, vpn: usize) {
        // TODO: 将 entries[vpn] 设为 None
        // 任务：将 self.entries[vpn] 设置为 None
        self.entries[vpn] = None;
    }

    // ------------------------------------------------------------------------
    // 实验 3: 查找页表项 (lookup)
    // 目标: 根据虚拟页号查找对应的页表项。
    // ------------------------------------------------------------------------

    /// 查询虚拟页号 `vpn` 对应的页表项。
    /// Lookup the page table entry for virtual page number `vpn`.
    pub fn lookup(&self, vpn: usize) -> Option<&PageTableEntry> {
        // TODO: 返回 entries[vpn] 的引用（如果存在）
        // 任务：使用 as_ref() 返回 Option<&PageTableEntry>
        self.entries[vpn].as_ref()
    }

    // ------------------------------------------------------------------------
    // 实验 4: 地址翻译 (translate)
    // 目标: 模拟 MMU，将虚拟地址转换为物理地址，并进行权限检查。
    // ------------------------------------------------------------------------

    /// 将虚拟地址翻译为物理地址。
    /// Translate a virtual address to a physical address.
    ///
    /// 步骤 / Steps：
    /// 1. 从虚拟地址中提取 VPN（高 20 位）和 offset（低 12 位）
    ///    Extract VPN (high 20 bits) and offset (low 12 bits) from virtual address.
    /// 2. 用 VPN 查页表，如果未映射返回 PageFault
    ///    Lookup VPN in page table, return PageFault if unmapped.
    /// 3. 检查 PTE_VALID 标志，未置位返回 PageFault
    ///    Check PTE_VALID flag, return PageFault if not set.
    /// 4. 如果 `is_write` 为 true，检查 PTE_WRITE 标志，未置位返回 PermissionDenied
    ///    If `is_write` is true, check PTE_WRITE flag, return PermissionDenied if not set.
    /// 5. 计算物理地址 = ppn * PAGE_SIZE + offset
    ///    Calculate physical address = ppn * PAGE_SIZE + offset.
    pub fn translate(&self, va: u32, is_write: bool) -> TranslateResult {
        // TODO: 实现虚拟地址到物理地址的翻译
        // 提示：
        //   let vpn = va_to_vpn(va);
        //   let offset = va_to_offset(va);
        //   使用 self.lookup(vpn) 获取页表项
        let vpn = va_to_vpn(va);
        let offset = va_to_offset(va);
        let pte = self.lookup(vpn);
        match pte {
            Some(entry) => {
                if entry.flags & PTE_VALID == 0 {
                    TranslateResult::PageFault
                } else if is_write && entry.flags & PTE_WRITE == 0 {
                    TranslateResult::PermissionDenied
                } else {
                    TranslateResult::Ok(make_pa(entry.ppn, offset))
                }
            }
            None => TranslateResult::PageFault,
        }
    }
}

// ------------------------------------------------------------------------
// 实验 5: 地址解析辅助函数 (va_to_vpn, va_to_offset, make_pa)
// 目标: 实现虚拟地址的拆解和物理地址的拼接。
// ------------------------------------------------------------------------

/// 从虚拟地址中提取虚拟页号。
/// Extract virtual page number from virtual address.
///
/// 提示：右移 PAGE_OFFSET_BITS 位。
/// Hint: Right shift by PAGE_OFFSET_BITS.
pub fn va_to_vpn(va: u32) -> usize {
    (va >> PAGE_OFFSET_BITS) as usize
}

/// 从虚拟地址中提取页内偏移。
/// Extract page offset from virtual address.
///
/// 提示：用掩码提取低 PAGE_OFFSET_BITS 位。
/// Hint: Use a mask to extract the lower PAGE_OFFSET_BITS.
pub fn va_to_offset(va: u32) -> u32 {
    va & ((1 << PAGE_OFFSET_BITS) - 1)
}

/// 由物理页号和偏移量拼出物理地址。
/// Construct physical address from physical page number and offset.
pub fn make_pa(ppn: u32, offset: u32) -> u32 {
    // TODO
    // 任务：实现 ppn * PAGE_SIZE + offset (或者 ppn << PAGE_OFFSET_BITS | offset)
    ppn << PAGE_OFFSET_BITS | offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_va_decompose() {
        // 虚拟地址 0x12345678
        // VPN = 0x12345, offset = 0x678
        assert_eq!(va_to_vpn(0x12345678), 0x12345);
        assert_eq!(va_to_offset(0x12345678), 0x678);
    }

    #[test]
    fn test_va_decompose_zero() {
        assert_eq!(va_to_vpn(0), 0);
        assert_eq!(va_to_offset(0), 0);
    }

    #[test]
    fn test_va_decompose_page_boundary() {
        // 正好在页边界，offset 应为 0
        assert_eq!(va_to_vpn(0x3000), 3);
        assert_eq!(va_to_offset(0x3000), 0);
    }

    #[test]
    fn test_make_pa() {
        assert_eq!(make_pa(0x80, 0x100), 0x80 * 4096 + 0x100);
        assert_eq!(make_pa(0, 0), 0);
        assert_eq!(make_pa(1, 0), 4096);
    }

    #[test]
    fn test_map_and_lookup() {
        let mut pt = SingleLevelPageTable::new(1024);
        pt.map(5, 100, PTE_VALID | PTE_READ);

        let entry = pt.lookup(5).expect("应该找到映射");
        assert_eq!(entry.ppn, 100);
        assert_eq!(entry.flags, PTE_VALID | PTE_READ);
    }

    #[test]
    fn test_lookup_unmapped() {
        let pt = SingleLevelPageTable::new(1024);
        assert!(pt.lookup(0).is_none());
    }

    #[test]
    fn test_unmap() {
        let mut pt = SingleLevelPageTable::new(1024);
        pt.map(10, 200, PTE_VALID | PTE_READ);
        assert!(pt.lookup(10).is_some());

        pt.unmap(10);
        assert!(pt.lookup(10).is_none());
    }

    #[test]
    fn test_translate_basic() {
        let mut pt = SingleLevelPageTable::new(1024);
        // 虚拟页 1 -> 物理页 0x80
        pt.map(1, 0x80, PTE_VALID | PTE_READ);

        // VA = 页1 + offset 0x100 = 0x1100
        let result = pt.translate(0x1100, false);
        // PA = 0x80 * 4096 + 0x100 = 0x80100
        assert_eq!(result, TranslateResult::Ok(0x80100));
    }

    #[test]
    fn test_translate_page_fault() {
        let pt = SingleLevelPageTable::new(1024);
        assert_eq!(pt.translate(0x5000, false), TranslateResult::PageFault);
    }

    #[test]
    fn test_translate_write_permission() {
        let mut pt = SingleLevelPageTable::new(1024);
        // 只读页
        pt.map(2, 0x90, PTE_VALID | PTE_READ);

        // 读取应成功
        assert_eq!(
            pt.translate(0x2000, false),
            TranslateResult::Ok(0x90 * PAGE_SIZE as u32)
        );
        // 写入应拒绝
        assert_eq!(
            pt.translate(0x2000, true),
            TranslateResult::PermissionDenied
        );
    }

    #[test]
    fn test_translate_writable_page() {
        let mut pt = SingleLevelPageTable::new(1024);
        pt.map(3, 0xA0, PTE_VALID | PTE_READ | PTE_WRITE);

        // 写入可写页应成功
        assert_eq!(
            pt.translate(0x3456, true),
            TranslateResult::Ok(0xA0 * PAGE_SIZE as u32 + 0x456)
        );
    }

    #[test]
    fn test_translate_invalid_entry() {
        let mut pt = SingleLevelPageTable::new(1024);
        // 映射了但 VALID 未置位
        pt.map(4, 0x50, PTE_READ);
        assert_eq!(pt.translate(0x4000, false), TranslateResult::PageFault);
    }

    #[test]
    fn test_multiple_mappings() {
        let mut pt = SingleLevelPageTable::new(1024);
        pt.map(0, 0x10, PTE_VALID | PTE_READ);
        pt.map(1, 0x20, PTE_VALID | PTE_READ | PTE_WRITE);
        pt.map(2, 0x30, PTE_VALID | PTE_READ);

        assert_eq!(pt.translate(0x0FFF, false), TranslateResult::Ok(0x10FFF));
        assert_eq!(pt.translate(0x1000, true), TranslateResult::Ok(0x20000));
        assert_eq!(pt.translate(0x2800, false), TranslateResult::Ok(0x30800));
    }
}
