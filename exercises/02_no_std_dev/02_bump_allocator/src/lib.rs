//! # Bump Allocator (no_std)
//!
//! Implement the simplest heap memory allocator: a Bump Allocator (bump pointer allocator).
//!
//! ## How It Works
//!
//! A Bump Allocator maintains a pointer `next` to the "next available address".
//! On each allocation, it aligns `next` to the requested alignment, then advances by `size` bytes.
//! It does not support freeing individual objects (`dealloc` is a no-op).
//!
//! ```text
//! heap_start                              heap_end
//! |----[allocated]----[allocated]----| next |---[free]---|
//!                                        ^
//!                                    next allocation starts here
//! ```
//!
//! ## Task
//!
//! Implement `BumpAllocator`'s `GlobalAlloc::alloc` method:
//! 1. Align the current `next` up to `layout.align()`
//!    Hint: `align_up(addr, align) = (addr + align - 1) & !(align - 1)`
//! 2. Check if the aligned address plus `layout.size()` exceeds `heap_end`
//! 3. If it exceeds, return `null_mut()`; otherwise atomically update `next` with `compare_exchange`
//!
//! ## Key Concepts
//!
//! - `core::alloc::{GlobalAlloc, Layout}`
//! - Memory alignment calculation
//! - `AtomicUsize` and `compare_exchange` (CAS loop)

// 当不是在测试环境下编译时，整个 crate 以 `no_std` 方式运行，
// 表示不能依赖 `std` 标准库（没有 OS 提供的堆、线程等），
// 只允许使用核心库 `core`，这和在真实内核/裸机环境下的场景一致。
#![cfg_attr(not(test), no_std)]

// 从 `core::alloc` 中引入全局分配器相关的 trait 和布局描述：
// - `GlobalAlloc`：实现它即可定义一个全局堆分配器
// - `Layout`：描述一次分配所需的大小和对齐要求
use core::alloc::{GlobalAlloc, Layout};
// `null_mut` 用来在分配失败时返回空指针，遵循全局分配器接口约定。
use core::ptr::null_mut;
// `AtomicUsize` 用原子方式维护“下一个可用地址”，保证在多线程环境下的安全更新；
// `Ordering` 指定原子操作的内存序，控制并发时的可见性和重排序。
use core::sync::atomic::{AtomicUsize, Ordering};

// =============================================================================
// 辅助函数：地址对齐
// =============================================================================

/// 将地址 `addr` 向上对齐到 `align` 的倍数。
///
/// # 数学含义
///
/// 我们要找最小的 `aligned >= addr` 且 `aligned % align == 0`。
/// 等价于：把 addr 向“高”的方向舍入到 align 的整数倍。
///
/// # 位运算公式
///
/// ```text
/// align_up(addr, align) = (addr + align - 1) & !(align - 1)
/// ```
///
/// - `align` 必须是 2 的幂（Rust 的 `Layout` 保证这一点）。
/// - `align - 1` 的二进制是低位全 1（例如 align=8 时，align-1=7 = 0b111）。
/// - `!(align - 1)` 是“掩码”：高位全 1、低 log2(align) 位为 0，用来把低位抹掉。
/// - `addr + align - 1` 先“多加上一点”，再和掩码做按位与，相当于向上舍入。
///
/// # 例子（align = 8）
///
/// - addr = 13 → (13+7) & !7 = 20 & ... = 16
/// - addr = 16 → (16+7) & !7 = 16
///
/// # 参数
///
/// - `addr`：当前“下一个可用”的地址（可能未对齐）。
/// - `align`：要求对齐的字节数（必须是 2 的幂，且通常 ≥ 1）。
///
/// # 返回
///
/// 大于等于 `addr` 且为 `align` 倍数的最小地址。
#[inline(always)]
fn align_up(addr: usize, align: usize) -> usize {
    // align 为 0 时会在运行时出问题；Layout 保证 align >= 1 且为 2 的幂，这里不额外断言。
    // 公式：(addr + align - 1) & !(align - 1)
    // !(align - 1) 在 Rust 中是对 usize 按位取反，得到的是“高位全 1、低位若干位为 0”的掩码。
    (addr + align - 1) & !(align - 1)
}

/// 递增式（bump）分配器的状态。
///
/// 整个堆空间被抽象为一段连续的内存区间 `[heap_start, heap_end)`：
/// - `heap_start`：堆的起始虚拟地址
/// - `heap_end`  ：堆的结束虚拟地址（最后一个可用字节之后的地址）
/// - `next`      ：“下一个可用地址”的原子变量，每次分配都会从这里开始向后挪动
///
/// 这种分配器只会不断向后“推进指针”，不支持单个块的释放（`dealloc` 不做任何事），
/// 但实现极其简单、开销很小，非常适合作为内核/运行时中的第一个堆分配器。
pub struct BumpAllocator {
    /// 堆空间起始地址（包含）
    heap_start: usize,
    /// 堆空间结束地址（不包含）
    heap_end: usize,
    /// 下一个可用字节的地址，使用原子类型以支持并发分配
    next: AtomicUsize,
}

impl BumpAllocator {
    /// Create a new BumpAllocator.
    ///
    /// # Safety
    /// `heap_start..heap_end` must be a valid, readable and writable memory region,
    /// and must not be used by other code during this allocator's lifetime.
    ///
    /// 安全约束（为什么是 `unsafe fn`）：
    /// - 调用者必须保证 `[heap_start, heap_end)` 这段地址范围在整个分配器生命周期内
    ///   始终指向一块**有效且可读写**的内存
    /// - 这段内存不能同时被其他代码拿去做别的用途，否则会产生数据竞争/内存破坏
    ///
    /// 这里使用 `const fn` 的原因是：在某些场景下可以在编译期构造一个静态的分配器实例，
    /// 例如作为全局分配器的静态变量。
    pub const unsafe fn new(heap_start: usize, heap_end: usize) -> Self {
        Self {
            heap_start,
            heap_end,
            // 初始化时，从堆起始地址开始作为“下一个可用地址”
            // 之后每次分配都会在此基础上向后 bump。
            next: AtomicUsize::new(heap_start),
        }
    }

    /// Reset the allocator (free all allocated memory).
    ///
    /// 将 `next` 重置回 `heap_start`，相当于“整体清空”整块堆空间，
    /// 但并不会对其中的数据做任何初始化或归零，只是逻辑上允许后续分配重新复用。
    ///
    /// 在真实 OS 中，这类“整体重置”通常用于：
    /// - 某个阶段性 Arena / 线程本地堆使用完毕后，一次性回收
    /// - 测试环境中反复复用同一段内存做分配实验
    pub fn reset(&self) {
        // 使用 SeqCst（顺序一致性）保证对所有线程都可见，并维持最强内存序约束，
        // 对这个简单场景来说虽然有些保守，但语义上最直观。
        self.next.store(self.heap_start, Ordering::SeqCst);
    }
}

// =============================================================================
// GlobalAlloc 实现：供 Rust 分配器接口调用
// =============================================================================

unsafe impl GlobalAlloc for BumpAllocator {
    /// 从堆中分配一块连续内存。
    ///
    /// # 参数
    ///
    /// - `layout`：描述本次分配的需求
    ///   - `layout.size()`：需要多少字节
    ///   - `layout.align()`：返回的地址必须是该值的倍数（2 的幂）
    ///
    /// # 返回值
    ///
    /// - 成功：指向一块大小至少为 `layout.size()`、对齐满足 `layout.align()` 的内存，
    ///   且这块内存在后续未被 dealloc 前，调用者可以独占使用。
    /// - 失败：返回 `null_mut()`（例如堆空间不足）。
    ///
    /// # 安全性（为什么是 unsafe fn）
    ///
    /// 调用者需保证：在调用 `alloc` 期间，`[heap_start, heap_end)` 仍然是有效、
    /// 可读写的内存区间，且与 `new` 时的安全约束一致。
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            // -------- 步骤 1：读取当前“下一个可用地址” --------
            // Ordering::SeqCst 表示“顺序一致性”：这次 load 相对于其他线程的原子操作
            // 有一个全局一致顺序，便于推理多线程行为。对分配器来说简单可靠。
            let old = self.next.load(Ordering::SeqCst);

            // -------- 步骤 2：将对齐要求应用到当前地址 --------
            // 返回的指针必须满足：ptr as usize % layout.align() == 0。
            // 所以从 old 开始“向上”舍入到 align 的倍数，得到本次分配的起始地址。
            let aligned = align_up(old, layout.align());

            // -------- 步骤 3：计算本次分配结束位置（不包含） --------
            // 我们打算把 [aligned, aligned + size) 这段区间交给调用者。
            // 若 aligned + size 发生数值溢出，下面与 heap_end 比较时可能出错；
            // 实际使用中堆大小有限，一般不会溢出。
            let end = aligned + layout.size();

            // -------- 步骤 4：边界检查，空间不足则返回空指针 --------
            // 若 end 超出堆尾，说明当前堆无法满足这次分配，返回 null_mut()，
            // 调用者（或更上层的分配逻辑）会据此处理 OOM。
            if end > self.heap_end {
                return null_mut();
            }

            // -------- 步骤 5：用 CAS 原子地“抢占”区间 [aligned, end) --------
            // compare_exchange(old, end, success_ord, failure_ord) 的含义：
            // - 若当前 self.next 的值等于 old，则把 self.next 设为 end，并返回 Ok(原值)
            // - 若当前 self.next 不等于 old（被其他线程改过），则返回 Err(当前值)，且不修改
            // 成功时：我们“抢到了”从 old 到 end 这一段，可以安全地把 aligned 返回给调用者。
            // 失败时：其他线程已经移动了 next，我们用新的 next 再跑一轮循环（重试）。
            match self.next.compare_exchange(old, end, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => {
                    // CAS 成功：当前线程是唯一把 next 从 old 更新为 end 的，区间 [aligned, end) 归本线程分配。
                    // 将起始地址转为 *mut u8 返回；调用者会在此地址上使用 layout.size() 字节。
                    return aligned as *mut u8;
                }
                Err(_) => {
                    // CAS 失败：其他线程在我们“读 old”和“写 end”之间修改了 next，产生竞争。
                    // 不返回错误，而是 continue 重新取 next、重新对齐、重新检查边界并 CAS，
                    // 直到某次成功或发现空间不足。
                    continue;
                }
            }
        }
    }

    /// 释放一块之前由 `alloc` 返回的内存。
    ///
    /// Bump 分配器不追踪单块分配，也不回收单块内存，因此这里是空实现。
    /// 只有在调用 `reset` 时才会逻辑上“清空”整块堆（把 next 拉回 heap_start）。
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // 故意留空：bump allocator 不回收单个对象。
    }
}

// ============================================================
// Tests
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    const HEAP_SIZE: usize = 4096;

    fn make_allocator() -> (BumpAllocator, Vec<u8>) {
        let mut heap = vec![0u8; HEAP_SIZE];
        let start = heap.as_mut_ptr() as usize;
        let alloc = unsafe { BumpAllocator::new(start, start + HEAP_SIZE) };
        (alloc, heap)
    }

    #[test]
    fn test_alloc_basic() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(16, 8).unwrap();
        let ptr = unsafe { alloc.alloc(layout) };
        assert!(!ptr.is_null(), "allocation should succeed");
    }

    #[test]
    fn test_alloc_alignment() {
        let (alloc, _heap) = make_allocator();
        for align in [1, 2, 4, 8, 16, 64] {
            let layout = Layout::from_size_align(1, align).unwrap();
            let ptr = unsafe { alloc.alloc(layout) };
            assert!(!ptr.is_null());
            assert_eq!(
                ptr as usize % align,
                0,
                "returned address must satisfy align={align}"
            );
        }
    }

    #[test]
    fn test_alloc_no_overlap() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(64, 8).unwrap();
        let p1 = unsafe { alloc.alloc(layout) } as usize;
        let p2 = unsafe { alloc.alloc(layout) } as usize;
        assert!(
            p1 + 64 <= p2 || p2 + 64 <= p1,
            "two allocations must not overlap"
        );
    }

    #[test]
    fn test_alloc_oom() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(HEAP_SIZE + 1, 1).unwrap();
        let ptr = unsafe { alloc.alloc(layout) };
        assert!(ptr.is_null(), "should return null when exceeding heap");
    }

    #[test]
    fn test_alloc_fill_heap() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(256, 1).unwrap();
        for i in 0..16 {
            let ptr = unsafe { alloc.alloc(layout) };
            assert!(!ptr.is_null(), "allocation #{i} should succeed");
        }
        let ptr = unsafe { alloc.alloc(layout) };
        assert!(ptr.is_null(), "should return null when heap is full");
    }

    #[test]
    fn test_reset() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(HEAP_SIZE, 1).unwrap();
        let p1 = unsafe { alloc.alloc(layout) };
        assert!(!p1.is_null());
        alloc.reset();
        let p2 = unsafe { alloc.alloc(layout) };
        assert!(!p2.is_null(), "should be able to allocate after reset");
        assert_eq!(
            p1, p2,
            "address after reset should match the first allocation"
        );
    }
}
