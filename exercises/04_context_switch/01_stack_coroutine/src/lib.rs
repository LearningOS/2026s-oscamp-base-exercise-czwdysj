//! # Stackful Coroutine and Context Switch (riscv64)
//! # 有栈协程与上下文切换 (riscv64)
//!
//! In this exercise, you implement the minimal context switch using inline assembly,
//! which is the core mechanism of OS thread scheduling. This crate is **riscv64 only**;
//! run `cargo test` on riscv64 Linux, or use the repo's normal flow (`./check.sh` / `oscamp`) on x86 with QEMU.
//! 在本练习中，你将使用内联汇编实现最小化的上下文切换，这是操作系统线程调度的核心机制。
//! 本 crate 仅支持 **riscv64** 架构；请在 riscv64 Linux 上运行 `cargo test`，
//! 或者在 x86 上使用仓库的标准流程（`./check.sh` / `oscamp`）通过 QEMU 运行。
//!
//! ## Key Concepts
//! ## 核心概念
//! - **Callee-saved registers**: Save and restore them on switch so the switched-away task can resume correctly later.
//!   **被调用者保存寄存器 (Callee-saved registers)**：在切换时保存并恢复它们，以便被切走的工作稍后能正确恢复执行。
//! - **Stack pointer `sp`** and **return address `ra`**: Restore them in the new context; the first time we switch to a task, `ret` jumps to `ra` (the entry point).
//!   **栈指针 `sp`** 和 **返回地址 `ra`**：在新的上下文中恢复它们；第一次切换到任务时，`ret` 指令会跳转到 `ra`（即入口点）。
//! - **Inline assembly**: `core::arch::asm!` / **内联汇编**：`core::arch::asm!`
//!
//! ## riscv64 ABI (for this exercise)
//! ## riscv64 ABI (本练习相关)
//! - Callee-saved: `sp`, `ra`, `s0`–`s11`. The `ret` instruction is `jalr zero, 0(ra)`.
//!   被调用者保存寄存器：`sp`, `ra`, `s0`–`s11`。`ret` 指令等价于 `jalr zero, 0(ra)`。
//! - First and second arguments: `a0` (old context), `a1` (new context).
//!   第一和第二个参数：`a0`（旧上下文地址），`a1`（新上下文地址）。

#![cfg(target_arch = "riscv64")]

/// Saved register state for one task (riscv64). Layout must match the offsets used in the asm below:
/// `sp` at 0, `ra` at 8, then `s0`–`s11` at 16, 24, … 104.
/// 任务保存的寄存器状态 (riscv64)。布局必须与下方汇编中使用的偏移量匹配：
/// `sp` 在偏移 0 处, `ra` 在偏移 8 处, 接着 `s0`–`s11` 在 16, 24, … 104 处。
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TaskContext {
    pub sp: u64,
    pub ra: u64,
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
}

impl TaskContext {
    /// Create an empty context
    /// 创建一个空的上下文
    pub const fn empty() -> Self {
        Self {
            sp: 0,
            ra: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }

    // ------------------------------------------------------------------------
    // 实验 1: 初始化上下文 (init)
    // 目标: 设置初始的栈指针和返回地址。
    // ------------------------------------------------------------------------

    /// Initialize this context so that when we switch to it, execution starts at `entry`.
    /// 初始化此上下文，以便当我们切换到它时，从 `entry` 开始执行。
    ///
    /// - Set `ra = entry` so that the first `ret` in the new context jumps to `entry`.
    /// - Set `sp = stack_top` with 16-byte alignment (RISC-V ABI requires 16-byte aligned stack at function entry).
    /// - Leave `s0`–`s11` zero; they will be loaded on switch.
    ///
    /// - 设置 `ra = entry`，使得新上下文中的第一个 `ret` 跳转到 `entry`。
    /// - 设置 `sp = stack_top` 并确保 16 字节对齐（RISC-V ABI 要求在函数入口处栈必须 16 字节对齐）。
    /// - 保持 `s0`–`s11` 为零；它们将在切换时被加载。
    ///
    /// 提示：对齐可以使用 `stack_top & !15`。
    pub fn init(&mut self, stack_top: usize, entry: usize) {
        // Set initial return address to the entry point
        // 设置初始返回地址为入口点
        self.ra = entry as u64;
        // Set initial stack pointer, ensuring 16-byte alignment as per RISC-V ABI
        // 设置初始栈指针，并确保满足 RISC-V ABI 要求的 16 字节对齐
        self.sp = (stack_top & !15) as u64;
    }
}

// ------------------------------------------------------------------------
// 实验 2: 上下文切换汇编 (switch_context)
// 目标: 编写汇编代码来保存当前寄存器并恢复目标寄存器。
// ------------------------------------------------------------------------

/// Switch from `old` to `new` context: save current callee-saved regs into `old`, load from `new`, then `ret` (jumps to `new.ra`).
/// 从 `old` 切换到 `new` 上下文：将当前被调用者保存寄存器存入 `old`，从 `new` 加载，然后 `ret`（跳转到 `new.ra`）。
///
/// In asm: store `sp`, `ra`, `s0`–`s11` to `[a0]` (old), load from `[a1]` (new), zero `a0`/`a1` so we do not leak pointers into the new context, then `ret`.
///
/// Must be `#[unsafe(naked)]` to prevent the compiler from generating a prologue/epilogue.
///
/// 在汇编中：将 `sp`, `ra`, `s0`–`s11` 存入 `[a0]` (old 指向的内存)，从 `[a1]` (new 指向的内存) 加载，
/// 将 `a0`/`a1` 清零以防指针泄露到新上下文中，然后执行 `ret`。
///
/// 必须使用 `#[unsafe(naked)]` 以防止编译器生成函数前导/后续代码（prologue/epilogue）。
///
/// 建议使用指令: sd (store doubleword), ld (load doubleword), ret
/// 寄存器参数：a0 = old 的地址, a1 = new 的地址
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old: &mut TaskContext, new: &TaskContext) {
    core::arch::naked_asm!(
        // Save old context to [a0]
        // 保存旧上下文到 [a0]
        "sd sp, 0(a0)",
        "sd ra, 8(a0)",
        "sd s0, 16(a0)",
        "sd s1, 24(a0)",
        "sd s2, 32(a0)",
        "sd s3, 40(a0)",
        "sd s4, 48(a0)",
        "sd s5, 56(a0)",
        "sd s6, 64(a0)",
        "sd s7, 72(a0)",
        "sd s8, 80(a0)",
        "sd s9, 88(a0)",
        "sd s10, 96(a0)",
        "sd s11, 104(a0)",
        // Load new context from [a1]
        // 从 [a1] 加载新上下文
        "ld sp, 0(a1)",
        "ld ra, 8(a1)",
        "ld s0, 16(a1)",
        "ld s1, 24(a1)",
        "ld s2, 32(a1)",
        "ld s3, 40(a1)",
        "ld s4, 48(a1)",
        "ld s5, 56(a1)",
        "ld s6, 64(a1)",
        "ld s7, 72(a1)",
        "ld s8, 80(a1)",
        "ld s9, 88(a1)",
        "ld s10, 96(a1)",
        "ld s11, 104(a1)",
        // Zero out a0 and a1 to avoid leaking pointers
        // 将 a0 和 a1 清零，避免指针泄露
        "li a0, 0",
        "li a1, 0",
        // Jump to the return address (new.ra)
        // 跳转到返回地址 (new.ra)
        "ret",
    );
}

// ------------------------------------------------------------------------
// 实验 3: 分配栈空间 (alloc_stack)
// 目标: 为协程分配一段内存作为运行栈。
// ------------------------------------------------------------------------

const STACK_SIZE: usize = 1024 * 64;

/// Allocate a stack for a coroutine. Returns `(buffer, stack_top)` where `stack_top` is the high address
/// (stack grows down). The buffer must be kept alive for the lifetime of the context using this stack.
/// 为协程分配一个栈。返回 `(buffer, stack_top)`，其中 `stack_top` 是高地址（栈向下增长）。
/// buffer 必须在上下文使用该栈的整个生命周期内保持存活。
///
/// 提示：使用 `vec![0u8; STACK_SIZE]` 分配，`stack_top` 是 buffer 末尾的地址。
/// 记得确保 `stack_top` 满足 16 字节对齐。
pub fn alloc_stack() -> (Vec<u8>, usize) {
    // Allocate buffer for the stack
    // 为栈分配缓冲区
    let mut buffer = vec![0u8; STACK_SIZE];
    // Calculate stack top address (high address, as RISC-V stack grows down)
    // 计算栈顶地址（高地址，因为 RISC-V 栈向下增长）
    let stack_top = buffer.as_ptr() as usize + STACK_SIZE;
    // Align stack top to 16 bytes
    // 确保栈顶地址 16 字节对齐
    let aligned_top = stack_top & !15;
    (buffer, aligned_top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    extern "C" fn task_entry() {
        COUNTER.store(42, Ordering::SeqCst);
        loop {
            std::hint::spin_loop();
        }
    }

    #[test]
    fn test_alloc_stack() {
        let (buf, top) = alloc_stack();
        assert_eq!(top, buf.as_ptr() as usize + STACK_SIZE);
        assert!(top % 16 == 0);
    }

    #[test]
    fn test_context_init() {
        let (buf, top) = alloc_stack();
        let _ = buf;
        let mut ctx = TaskContext::empty();
        let entry = task_entry as *const () as usize;
        ctx.init(top, entry);
        assert_eq!(ctx.ra, entry as u64);
        assert!(ctx.sp != 0);
    }

    #[test]
    fn test_switch_to_task() {
        COUNTER.store(0, Ordering::SeqCst);

        static mut MAIN_CTX_PTR: *mut TaskContext = std::ptr::null_mut();
        static mut TASK_CTX_PTR: *mut TaskContext = std::ptr::null_mut();

        extern "C" fn cooperative_task() {
            COUNTER.store(99, Ordering::SeqCst);
            unsafe {
                switch_context(&mut *TASK_CTX_PTR, &*MAIN_CTX_PTR);
            }
        }

        let (_stack_buf, stack_top) = alloc_stack();
        let mut main_ctx = TaskContext::empty();
        let mut task_ctx = TaskContext::empty();
        task_ctx.init(stack_top, cooperative_task as *const () as usize);

        unsafe {
            MAIN_CTX_PTR = &mut main_ctx;
            TASK_CTX_PTR = &mut task_ctx;
            switch_context(&mut main_ctx, &task_ctx);
        }

        assert_eq!(COUNTER.load(Ordering::SeqCst), 99);
    }
}
