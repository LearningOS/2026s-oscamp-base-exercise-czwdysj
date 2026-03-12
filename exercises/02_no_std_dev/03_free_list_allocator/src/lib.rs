//! # Free-List Allocator
//!
//! 在 bump allocator 的基础上，实现一个**带内存回收能力**的 Free-List 分配器。
//!
//! ## 工作原理（高层）
//!
//! - 使用一条**单向链表**记录当前所有“空闲块”（free blocks）
//! - `alloc` 时：
//!   1. 先在 free_list 里从头到尾查找**第一块够大、且满足对齐要求**的空闲块（first‑fit）
//!   2. 找到就把它**从链表摘下**，整个块直接返回给调用者
//!   3. 找不到，就像 bump allocator 一样，从尚未使用过的“bump 区”继续往后切一块出来
//! - `dealloc` 时：
//!   1. 在被释放的内存块开头写入一个 `FreeBlock` 头部（size + next）
//!   2. 把它插入到 free_list 链表头（LIFO）
//!
//! ```text
//! free_list -> [block A: 64B] -> [block B: 128B] -> [block C: 32B] -> null
//! ```
//!
//! 每个空闲块在自己的起始位置嵌入一个 `FreeBlock` 结构体，用来描述该块的大小和指向下一块的指针，
//! 这是一种 **intrusive linked list（侵入式链表）**：节点数据就长在节点本身的内存里。
//!
//! ## 实验任务
//!
//! 你需要实现 `FreeListAllocator` 的 `alloc` 和 `dealloc`：
//!
//! ### `alloc`
//! 1. 遍历 free_list，按 first‑fit 寻找第一块满足
//!    - 块起始地址满足对齐要求
//!    - 块大小 `>= layout.size()`（这里我们还会向上取整到至少 `sizeof(FreeBlock)`）
//! 2. 若找到：
//!    - 从链表中移除该块
//!    - 返回块起始地址作为分配结果
//! 3. 若找不到：
//!    - 回退到 bump 分配策略：从 `bump_next` 开始，对齐并向后“推指针”
//!
//! ### `dealloc`
//! 1. 将 `ptr` 视为一个新的 `FreeBlock` 头部，把 size 写进去
//! 2. 将其 `next` 指向当前 free_list 头
//! 3. 更新 free_list 头为该块，实现“头插法”
//!
//! ## 关键知识点
//!
//! - 侵入式链表（intrusive linked list）
//! - 裸指针 `*mut T` 的读写：`ptr.read()` / `ptr.write(val)`，以及通过 `(*ptr).field` 访问
//! - 内存对齐检查（`addr % align == 0`）和对齐计算（`align_up`）

#![cfg_attr(not(test), no_std)]

// `GlobalAlloc` / `Layout` 是 Rust 自定义分配器的核心接口：
// - `GlobalAlloc`：实现它即可接管 `Box` / `Vec` 等堆分配请求（在本实验里我们手写 alloc/dealloc）
// - `Layout`：一次分配请求的“合同”，包含 size（大小）与 align（对齐）
use core::alloc::{GlobalAlloc, Layout};
// 约定：当分配失败（OOM）时，返回一个空指针 `null_mut()`。
use core::ptr::null_mut;
// bump 区域使用 `AtomicUsize` + 内存序控制，实现无锁并发 bump 分配。
use core::sync::atomic::{AtomicUsize, Ordering};

// =============================================================================
// 辅助函数：地址向上对齐
// =============================================================================

/// 将地址 `addr` 向上对齐到 `align` 的倍数。
///
/// 要求：`align` 为 2 的幂（由 `Layout` 保证），且 `align >= 1`。
///
/// 计算公式：
/// ```text
/// align_up(addr, align) = (addr + align - 1) & !(align - 1)
/// ```
/// - `align - 1`：低位全 1（例如 align = 8 → 0b111）
/// - `!(align - 1)`：掩码，高位全 1、低位若干位为 0，用于“抹掉”低位
/// - `addr + align - 1`：先向上“超一点”，再用掩码砍掉低位，相当于向上取整到 align 的倍数
#[inline(always)]
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// 空闲块（free block）的头部。
///
/// 在 free-list 分配器里，“空闲内存”通常被组织成一条链表：
/// 每一段空闲块的起始位置，都会放一个小小的头部（header），
/// 用来记录这块空闲内存的长度，以及下一块空闲内存的位置。
///
/// 关键点：这个结构体本身就直接“存放在那块空闲内存里”，
/// 也就是说当一块内存被释放后，我们会把它的开头几个字节当作 `FreeBlock` 来写入元数据。
struct FreeBlock {
    /// 这一块空闲块的大小（以字节为单位）。
    /// 约定上通常至少要能容纳一个 `FreeBlock` 头部本身，否则无法把它挂回链表。
    size: usize,
    /// 指向下一块空闲块头部的指针（单向链表）。
    ///
    /// 这里用裸指针 `*mut FreeBlock`，因为分配器要在“原始内存”上操作；
    /// 在 `no_std` / 内核风格代码里通常不会用 `Box`/`Vec` 来存这些元数据。
    next: *mut FreeBlock,
}

pub struct FreeListAllocator {
    /// 堆起始地址（包含）。
    heap_start: usize,
    heap_end: usize,
    /// “bump 指针”：尚未被使用过的“新鲜”内存从这里开始。
    ///
    /// Free-list 分配器常见做法是“两条路并行”：
    /// - 先尝试从 `free_list`（已释放回收的块）里找合适的块复用
    /// - 如果 free_list 找不到，再从 bump 区域（还没用过的堆尾部）切一块出来
    ///
    /// 用 `AtomicUsize`（而不是普通 usize）的原因：
    /// - bump 分配在并发环境下也可能被多个线程同时调用
    /// - 用原子 CAS/原子更新可以避免两次分配拿到重叠的区间
    bump_next: AtomicUsize,
    /// 空闲链表的表头指针。
    ///
    /// 这里故意把“测试环境”和“非测试环境”分开实现：
    ///
    /// - 测试（`cfg(test)`）下可以用 `std::sync::Mutex`：
    ///   - 写起来更安全（不会数据竞争）
    ///   - 测试在宿主机上跑，允许用 `std`
    ///
    /// - 非测试（`not(test)`）下不能依赖 `std`，因此用 `core::cell::UnsafeCell`：
    ///   - `UnsafeCell<T>` 是 Rust 提供的“内部可变性”原语，允许在 `&self` 下修改内部数据
    ///   - 但它不提供同步保证：并发安全需要你在更高层自己加锁（比如自旋锁/关中断等）
    ///
    /// 换句话说：`UnsafeCell` 只是让“可变”变得可能，至于“并发安全”，要靠外部协议保证。
    #[cfg(test)]
    free_list: std::sync::Mutex<*mut FreeBlock>,
    #[cfg(not(test))]
    free_list: core::cell::UnsafeCell<*mut FreeBlock>,
}

// -----------------------------------------------------------------------------
// Send / Sync 的 unsafe 实现
// -----------------------------------------------------------------------------
// 这个分配器内部含有裸指针与 `UnsafeCell`，编译器无法自动证明它满足线程安全要求，
// 所以我们需要 `unsafe impl Send/Sync` 来“向编译器承诺”：
// - Send：这个类型可以在线程间移动
// - Sync：对 `&FreeListAllocator` 的共享引用可以跨线程使用
//
// 这是一种常见的系统编程模式：当你用原语（原子/锁/关中断）自行保证并发正确性时，
// 需要用 unsafe impl 把这件事告诉 Rust 的类型系统。
//
// 注意：这里分成 test/非 test 两套 cfg，只是为了适配字段类型不同（Mutex vs UnsafeCell），
// 语义上都是“我们保证它可跨线程使用”。
#[cfg(test)]
unsafe impl Send for FreeListAllocator {}
#[cfg(test)]
unsafe impl Sync for FreeListAllocator {}
#[cfg(not(test))]
unsafe impl Send for FreeListAllocator {}
#[cfg(not(test))]
unsafe impl Sync for FreeListAllocator {}

impl FreeListAllocator {
    /// # Safety
    /// `heap_start..heap_end` must be a valid readable and writable memory region.
    pub unsafe fn new(heap_start: usize, heap_end: usize) -> Self {
        Self {
            heap_start,
            heap_end,
            // bump_next 从 heap_start 起步，表示“未使用过的新内存”从堆起点开始。
            // 后续每次从 bump 区域分配，会把这个指针向后移动。
            bump_next: AtomicUsize::new(heap_start),
            #[cfg(test)]
            // 测试环境：用 Mutex 包住 free_list 头指针，避免测试用例并发时产生数据竞争。
            free_list: std::sync::Mutex::new(null_mut()),
            #[cfg(not(test))]
            // 非测试环境：没有 std::sync::Mutex 可用，先用 UnsafeCell 存放头指针。
            // 真正的并发互斥应由外部提供（例如内核里的自旋锁）。
            free_list: core::cell::UnsafeCell::new(null_mut()),
        }
    }

    #[cfg(test)]
    fn free_list_head(&self) -> *mut FreeBlock {
        // Mutex::lock() 返回一个 guard，离开作用域自动解锁。
        // 这里解引用 guard，得到当前链表头指针的值（复制出来返回）。
        *self.free_list.lock().unwrap()
    }

    #[cfg(test)]
    fn set_free_list_head(&self, head: *mut FreeBlock) {
        // 同样在锁保护下写入新的头指针。
        *self.free_list.lock().unwrap() = head;
    }

    #[cfg(not(test))]
    fn free_list_head(&self) -> *mut FreeBlock {
        // 非测试环境：直接从 UnsafeCell 里读出头指针。
        // 这是 unsafe 的原因：UnsafeCell 绕开了 Rust 的借用规则；并发正确性靠外部协议保证。
        unsafe { *self.free_list.get() }
    }

    #[cfg(not(test))]
    fn set_free_list_head(&self, head: *mut FreeBlock) {
        // 非测试环境：直接写入头指针。
        // 同样需要调用者/上层在并发时自行保证互斥。
        unsafe { *self.free_list.get() = head }
    }
}

unsafe impl GlobalAlloc for FreeListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // ---------------------------------------------------------------------
        // 预处理：对齐 size 和 align，保证将来能放下一个 FreeBlock 头部
        // ---------------------------------------------------------------------
        // 约定：无论用户请求多小，我们实际分配的块至少要能容纳一个 `FreeBlock`，
        // 这样在 dealloc 时才能把整块挂回 free_list（头部就写在块起始处）。
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());
        // 对齐要求同理：块起始地址至少要满足 `FreeBlock` 自身的对齐，
        // 以便未来把它转换成 `*mut FreeBlock` 安全地读写字段。
        let align = layout.align().max(core::mem::align_of::<FreeBlock>());

        // ---------------------------------------------------------------------
        // Step 1：在 free_list 中按 first‑fit 查找合适的空闲块
        // ---------------------------------------------------------------------
        //
        // 这里采用最简单的策略：
        // - 只要“块起始地址满足对齐 & 块大小 >= size”，就直接整块拿走
        // - 不做块切分（即便只用掉一部分，剩余的碎片也暂时浪费掉）
        //
        // 这样虽然会有一些外部碎片，但实现逻辑简单、足以通过本实验的测试。

        // `prev` 指向当前 `curr` 之前的一个节点（用于从链表中摘除 curr）
        let mut prev: *mut FreeBlock = null_mut();
        // 当前正在检查的空闲块
        let mut curr = self.free_list_head();

        while !curr.is_null() {
            // 当前块的起始地址
            let curr_addr = curr as usize;
            // 检查对齐：要求块起始地址本身就满足对齐
            // 若不满足，可以选择跳过此块；复杂实现可以尝试在块中间找对齐位置并切分，
            // 但这里为了简洁直接略过不对齐的块。
            if curr_addr % align == 0 {
                // 安全前提：curr 来源于 free_list，指向一块之前分配过、大小至少为 size 的区域。
                let block_size = (*curr).size;
                if block_size >= size {
                    // 找到了第一块满足条件的空闲块（first‑fit 命中）。

                    // 从链表中移除 curr：
                    // - 若 prev 为空，说明命中的是链表头，需要更新 free_list_head
                    // - 否则，把 prev.next 指向 curr.next
                    let next = (*curr).next;
                    if prev.is_null() {
                        // 命中头节点
                        self.set_free_list_head(next);
                    } else {
                        // 命中中间节点
                        (*prev).next = next;
                    }

                    // 返回这块空间的起始地址作为结果。
                    // 注意：我们把整个块都交给调用者使用；在其生命周期内，
                    // 这块内存不再属于 free_list，因此头部字段内容可以被用户覆盖。
                    return curr_addr as *mut u8;
                }
            }

            // 继续向后遍历
            prev = curr;
            curr = (*curr).next;
        }

        // ---------------------------------------------------------------------
        // Step 2：free_list 无合适块 → 退回到 bump 分配策略
        // ---------------------------------------------------------------------
        //
        // 使用与上一实验类似的 CAS 循环：
        // - 从 bump_next 读出当前“下一个可用地址” old
        // - 对齐到 align：aligned = align_up(old, align)
        // - 计算 end = aligned + size，检查是否越过 heap_end
        // - 用 compare_exchange(old, end, ...) 尝试原子更新 bump_next
        //   * 成功 → 这段 [aligned, end) 归当前线程所有，返回 aligned
        //   * 失败 → 有别的线程抢先更新了 bump_next，读取新值重试

        loop {
            // 当前 bump 指针的值
            let old = self.bump_next.load(Ordering::SeqCst);

            // 若 bump 指针已经跑出了堆范围，直接 OOM
            if old >= self.heap_end {
                return null_mut();
            }

            // 对齐起始地址
            let aligned = align_up(old, align);
            let end = aligned.saturating_add(size);

            // 边界检查：超出堆空间则分配失败
            if end > self.heap_end || end < aligned {
                // end < aligned 通过 saturating_add 理论上不会发生，但多做一层保险。
                return null_mut();
            }

            // CAS 尝试把 bump_next 从 old 更新为 end
            match self
                .bump_next
                .compare_exchange(old, end, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => {
                    // 更新成功：当前线程成功“占有”了 [aligned, end) 这一段空间。
                    return aligned as *mut u8;
                }
                Err(_) => {
                    // 有其他线程在我们之间修改了 bump_next，重新读取 old 并重试。
                    continue;
                }
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // 同样为了保证能写入 FreeBlock 头部，把 size 至少扩展到 `size_of::<FreeBlock>()`。
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());

        // ---------------------------------------------------------------------
        // 将被释放的块挂回 free_list（头插法）
        // ---------------------------------------------------------------------
        //
        // 注意：这里假设调用者传入的 `ptr`：
        // - 来自当前分配器之前返回的某次 `alloc`
        // - 大小与 `layout` 一致（这是 `GlobalAlloc` 接口对调用者的要求）
        //
        // 我们直接把 `ptr` 视为一个 `FreeBlock` 头部所在位置，并在此写入元数据。

        // 1. 把用户指针转换成 `FreeBlock` 头部指针。
        let block = ptr as *mut FreeBlock;

        // 2. 读取当前 free_list 的表头，作为新块的 next 指针。
        let old_head = self.free_list_head();

        // 3. 在被释放的内存块起始位置写入 FreeBlock 结构。
        //
        //    这里使用“按字段赋值”：等价于 `block.write(FreeBlock { .. })`，但更直观。
        //    前提是：`ptr` 至少满足 FreeBlock 的对齐要求（我们在 alloc 时已经保证了）。
        (*block).size = size;
        (*block).next = old_head;

        // 4. 更新 free_list 头指针：新释放的块成为新的链表头。
        self.set_free_list_head(block);
    }
}

// ============================================================
// Tests
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    const HEAP_SIZE: usize = 4096;

    fn make_allocator() -> (FreeListAllocator, Vec<u8>) {
        let mut heap = vec![0u8; HEAP_SIZE];
        let start = heap.as_mut_ptr() as usize;
        let alloc = unsafe { FreeListAllocator::new(start, start + HEAP_SIZE) };
        (alloc, heap)
    }

    #[test]
    fn test_alloc_basic() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(32, 8).unwrap();
        let ptr = unsafe { alloc.alloc(layout) };
        assert!(!ptr.is_null());
    }

    #[test]
    fn test_alloc_alignment() {
        let (alloc, _heap) = make_allocator();
        for align in [1, 2, 4, 8, 16] {
            let layout = Layout::from_size_align(8, align).unwrap();
            let ptr = unsafe { alloc.alloc(layout) };
            assert!(!ptr.is_null());
            assert_eq!(ptr as usize % align, 0, "align={align}");
        }
    }

    #[test]
    fn test_dealloc_and_reuse() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(64, 8).unwrap();

        let p1 = unsafe { alloc.alloc(layout) };
        assert!(!p1.is_null());

        // After freeing, the next allocation should reuse the same block
        unsafe { alloc.dealloc(p1, layout) };
        let p2 = unsafe { alloc.alloc(layout) };
        assert!(!p2.is_null());
        assert_eq!(p1, p2, "should reuse the freed block");
    }

    #[test]
    fn test_multiple_alloc_dealloc() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(128, 8).unwrap();

        let p1 = unsafe { alloc.alloc(layout) };
        let p2 = unsafe { alloc.alloc(layout) };
        let p3 = unsafe { alloc.alloc(layout) };
        assert!(!p1.is_null() && !p2.is_null() && !p3.is_null());

        unsafe { alloc.dealloc(p2, layout) };
        unsafe { alloc.dealloc(p1, layout) };

        let q1 = unsafe { alloc.alloc(layout) };
        let q2 = unsafe { alloc.alloc(layout) };
        assert!(!q1.is_null() && !q2.is_null());
    }

    #[test]
    fn test_oom() {
        let (alloc, _heap) = make_allocator();
        let layout = Layout::from_size_align(HEAP_SIZE + 1, 1).unwrap();
        let ptr = unsafe { alloc.alloc(layout) };
        assert!(ptr.is_null(), "should return null when exceeding heap");
    }
}
