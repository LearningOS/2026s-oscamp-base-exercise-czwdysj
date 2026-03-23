//! # Green Thread Scheduler (riscv64)
//! # 绿色线程调度器 (riscv64)
//!
//! In this exercise, you build a simple cooperative (green) thread scheduler on top of context switching.
//! This crate is **riscv64 only**; run with the repo's normal flow (`./check.sh` / `oscamp`) or natively on riscv64.
//! 在本练习中，你将在上下文切换的基础上构建一个简单的协作式（绿色）线程调度器。
//! 本 crate 仅支持 **riscv64** 架构；请使用仓库的标准流程（`./check.sh` / `oscamp`）运行，或在 riscv64 原生环境下运行。
//!
//! ## Key Concepts
//! ## 核心概念
//! - **Cooperative vs preemptive scheduling** / **协作式与抢占式调度**
//! - **Thread state**: `Ready`, `Running`, `Finished` / **线程状态**：`就绪`、`运行中`、`已完成`
//! - `yield_now()`: current thread voluntarily gives up the CPU / `yield_now()`：当前线程主动放弃 CPU
//! - **Scheduler loop**: pick next ready thread and switch to it / **调度器循环**：选择下一个就绪线程并切换到它
//!
//! ## Design
//! ## 设计
//! Each green thread has its own stack and `TaskContext`. Threads call `yield_now()` to yield.
//! The scheduler round-robins among ready threads. User entry is wrapped by `thread_wrapper`, which
//! calls the entry then marks the thread `Finished` and switches back.
//! 每个绿色线程都有自己的栈和 `TaskContext`。线程调用 `yield_now()` 来让出执行权。
//! 调度器在就绪线程之间进行轮询（round-robin）。用户入口函数被 `thread_wrapper` 包装，
//! 该包装函数调用入口函数，然后将线程标记为 `已完成` 并切换回调度器。

#![cfg(target_arch = "riscv64")]

use core::arch::naked_asm;

/// Per-thread stack size. Slightly larger to avoid overflow under QEMU / test harness.
/// 每个线程的栈大小。略大一些以防止在 QEMU 或测试环境下溢出。
const STACK_SIZE: usize = 1024 * 128;

/// Task context (riscv64); layout must match `01_stack_coroutine::TaskContext` and the asm below.
/// 任务上下文 (riscv64)；布局必须与 `01_stack_coroutine::TaskContext` 以及下方的汇编匹配。
#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct TaskContext {
    sp: u64,
    ra: u64,
    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
    s10: u64,
    s11: u64,
}

/// Thread state
/// 线程状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreadState {
    Ready,
    Running,
    Finished,
}

/// Green thread structure
/// 绿色线程结构体
struct GreenThread {
    ctx: TaskContext,
    state: ThreadState,
    _stack: Option<Vec<u8>>,
    /// User entry; taken once when the thread is first scheduled and passed to `thread_wrapper`.
    /// 用户入口函数；在线程第一次被调度时提取一次，并传递给 `thread_wrapper`。
    entry: Option<extern "C" fn()>,
}

/// Set by the scheduler before switching to a new thread; `thread_wrapper` reads and calls it once.
/// 在切换到新线程之前由调度器设置；`thread_wrapper` 读取并调用它一次。
static mut CURRENT_THREAD_ENTRY: Option<extern "C" fn()> = None;

/// Wrapper run as the initial `ra` for each green thread: call the user entry (from `CURRENT_THREAD_ENTRY`), then mark Finished and switch back.
/// 每个绿色线程初始 `ra` 指向的包装函数：调用用户入口（从 `CURRENT_THREAD_ENTRY` 获取），然后标记为已完成并切换回调度器。
extern "C" fn thread_wrapper() {
    let entry = unsafe { core::ptr::read(&raw const CURRENT_THREAD_ENTRY) };
    if let Some(f) = entry {
        unsafe { CURRENT_THREAD_ENTRY = None };
        f();
    }
    thread_finished();
}

/// Save current callee-saved regs into `old`, load from `new`, then `ret` to `new.ra`.
/// Zero `a0`/`a1` before `ret` so we don't leak pointers into the new context.
///
/// Must be `#[unsafe(naked)]` to prevent the compiler from generating a prologue/epilogue.
///
/// 保存当前的被调用者保存寄存器到 `old`，从 `new` 加载，然后 `ret` 跳转到 `new.ra`。
/// 在 `ret` 之前将 `a0`/`a1` 清零，以防指针泄露到新上下文中。
///
/// 必须使用 `#[unsafe(naked)]` 以防止编译器生成函数前导/后续代码。
#[unsafe(naked)]
unsafe extern "C" fn switch_context(_old: &mut TaskContext, _new: &TaskContext) {
    naked_asm!(
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
        "li a0, 0",
        "li a1, 0",
        "ret",
    );
}

/// Scheduler structure
/// 调度器结构体
pub struct Scheduler {
    threads: Vec<GreenThread>,
    current: usize,
}

impl Scheduler {
    /// Create a new scheduler with a main thread
    /// 创建一个包含主线程的新调度器
    pub fn new() -> Self {
        let main_thread = GreenThread {
            ctx: TaskContext::default(),
            state: ThreadState::Running,
            _stack: None,
            entry: None,
        };

        Self {
            threads: vec![main_thread],
            current: 0,
        }
    }

    // ------------------------------------------------------------------------
    // 实验 1: 注册线程 (spawn)
    // 目标: 创建并注册一个新的绿色线程。
    // ------------------------------------------------------------------------

    /// Register a new green thread that will run `entry` when first scheduled.
    /// 注册一个新线程，该线程在第一次被调度时运行 `entry`。
    ///
    /// 1. Allocate a stack of `STACK_SIZE` bytes; compute `stack_top` (high address).
    /// 2. Set up the context: `ra = thread_wrapper` so the first switch jumps to the wrapper;
    ///    `sp` must be 16-byte aligned (e.g. `(stack_top - 16) & !15` to leave headroom).
    /// 3. Push a `GreenThread` with this context, state `Ready`, and `entry` stored for the wrapper to call.
    ///
    /// 1. 分配 `STACK_SIZE` 字节的栈；计算 `stack_top`（高地址）。
    /// 2. 设置上下文：`ra = thread_wrapper`，使得第一次切换时跳转到包装函数；
    ///    `sp` 必须 16 字节对齐（例如 `(stack_top - 16) & !15` 以留出空间）。
    /// 3. 压入一个 `GreenThread`，包含此上下文、状态为 `Ready`，并存储 `entry` 供包装函数调用。
    pub fn spawn(&mut self, entry: extern "C" fn()) {
        // 1. Allocate a private stack for the thread
        // 为线程分配私有栈空间
        let stack = vec![0u8; STACK_SIZE];
        let stack_top = stack.as_ptr() as usize + STACK_SIZE;

        // 2. Initialize the register context
        // 初始化寄存器上下文
        let mut ctx = TaskContext::default();
        // Set return address to the wrapper, which will call the user entry
        // 设置返回地址为包装函数，该函数将调用用户入口
        ctx.ra = thread_wrapper as u64;
        // Align stack pointer to 16 bytes as per RISC-V ABI requirements
        // 按照 RISC-V ABI 要求，确保栈指针 16 字节对齐
        ctx.sp = (stack_top & !15) as u64;

        // 3. Store the thread in the scheduler's list
        // 将线程存入调度器的列表中
        self.threads.push(GreenThread {
            ctx,
            state: ThreadState::Ready,
            _stack: Some(stack),
            entry: Some(entry),
        });
    }

    // ------------------------------------------------------------------------
    // 实验 2: 运行调度器 (run)
    // 目标: 启动调度循环。
    // ------------------------------------------------------------------------

    /// Run the scheduler until all threads (except the main one) are `Finished`.
    /// 运行调度器，直到所有线程（除了主线程）都变为 `已完成`。
    ///
    /// 1. Set the global `SCHEDULER` pointer to `self` so that `yield_now` and `thread_finished` can call back.
    /// 2. Loop: if all threads in `threads[1..]` are `Finished`, break; otherwise call `schedule_next()`.
    /// 3. Clear `SCHEDULER` when done.
    pub fn run(&mut self) {
        // Set the global static pointer so threads can access the scheduler
        // 设置全局静态指针，使线程能够访问调度器
        unsafe {
            SCHEDULER = self as *mut Scheduler;
        }

        // Loop until all user threads are done
        // 循环直到所有用户线程执行完毕
        while self
            .threads
            .iter()
            .skip(1)
            .any(|t| t.state != ThreadState::Finished)
        {
            self.schedule_next();
        }

        // Reset the global pointer when finished
        // 执行完毕后重置全局指针
        unsafe {
            SCHEDULER = std::ptr::null_mut();
        }
    }

    // ------------------------------------------------------------------------
    // 实验 3: 调度下一个就绪线程 (schedule_next)
    // 目标: 寻找下一个 Ready 线程并执行上下文切换。
    // ------------------------------------------------------------------------

    /// Find the next ready thread (starting from `current + 1` round-robin), mark current as `Ready` (if not `Finished`), mark next as `Running`, set `CURRENT_THREAD_ENTRY` if the next thread has an entry, then switch to it.
    /// 寻找下一个就绪线程（从 `current + 1` 开始轮询），将当前线程标记为 `Ready`（如果不是 `Finished`），
    /// 将下一个线程标记为 `Running`，如果下一个线程有入口函数则设置 `CURRENT_THREAD_ENTRY`，然后切换过去。
    ///
    /// 提示：使用 `switch_context` 进行切换。
    fn schedule_next(&mut self) {
        let old_idx = self.current;
        let mut next_idx = (old_idx + 1) % self.threads.len();

        // Round-robin: find the next thread that is 'Ready'
        // 轮询调度：寻找下一个处于 'Ready' 状态的线程
        while self.threads[next_idx].state != ThreadState::Ready {
            if next_idx == old_idx {
                // No other threads are ready to run
                // 没有其他就绪线程可以运行
                return;
            }
            next_idx = (next_idx + 1) % self.threads.len();
        }

        // Transition the current thread to Ready if it was Running
        // 如果当前线程正在运行，将其状态切回 Ready
        if self.threads[old_idx].state == ThreadState::Running {
            self.threads[old_idx].state = ThreadState::Ready;
        }

        // Mark the next thread as Running
        // 将下一个线程标记为运行中
        self.threads[next_idx].state = ThreadState::Running;
        self.current = next_idx;

        // If the thread is starting for the first time, provide its entry point to the wrapper
        // 如果线程是第一次启动，将其入口点提供给包装函数
        if let Some(entry) = self.threads[next_idx].entry.take() {
            unsafe {
                CURRENT_THREAD_ENTRY = Some(entry);
            }
        }

        // Execute the low-level context switch
        // 执行底层上下文切换
        let old_ctx_ptr = self.threads[old_idx].ctx.as_mut_ptr();
        let new_ctx_ptr = self.threads[next_idx].ctx.as_ptr();

        unsafe {
            switch_context(&mut *old_ctx_ptr, &*new_ctx_ptr);
        }
    }
}

impl TaskContext {
    fn as_mut_ptr(&mut self) -> *mut TaskContext {
        self as *mut TaskContext
    }
    fn as_ptr(&self) -> *const TaskContext {
        self as *const TaskContext
    }
}

static mut SCHEDULER: *mut Scheduler = std::ptr::null_mut();

/// Current thread voluntarily yields; the scheduler will pick the next ready thread.
pub fn yield_now() {
    unsafe {
        if !SCHEDULER.is_null() {
            (*SCHEDULER).schedule_next();
        }
    }
}

/// Mark current thread as `Finished` and switch to the next (called by `thread_wrapper` after the user entry returns).
fn thread_finished() {
    unsafe {
        if !SCHEDULER.is_null() {
            let sched = &mut *SCHEDULER;
            sched.threads[sched.current].state = ThreadState::Finished;
            sched.schedule_next();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    /// Tests must run serially: the scheduler uses global state (SCHEDULER, CURRENT_THREAD_ENTRY).
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    static EXEC_ORDER: AtomicU32 = AtomicU32::new(0);

    extern "C" fn task_a() {
        EXEC_ORDER.fetch_add(1, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(10, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(100, Ordering::SeqCst);
    }

    extern "C" fn task_b() {
        EXEC_ORDER.fetch_add(1, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(10, Ordering::SeqCst);
    }

    #[test]
    fn test_scheduler_runs_all() {
        let _guard = TEST_LOCK.lock().unwrap();
        EXEC_ORDER.store(0, Ordering::SeqCst);

        let mut sched = Scheduler::new();
        sched.spawn(task_a);
        sched.spawn(task_b);
        sched.run();

        let got = EXEC_ORDER.load(Ordering::SeqCst);
        if got != 122 {
            panic!(
                "EXEC_ORDER: expected 122, got {} (run with --nocapture to see stderr)",
                got
            );
        }
    }

    static SIMPLE_FLAG: AtomicU32 = AtomicU32::new(0);

    extern "C" fn simple_task() {
        SIMPLE_FLAG.store(42, Ordering::SeqCst);
    }

    #[test]
    fn test_single_thread() {
        let _guard = TEST_LOCK.lock().unwrap();
        SIMPLE_FLAG.store(0, Ordering::SeqCst);

        let mut sched = Scheduler::new();
        sched.spawn(simple_task);
        sched.run();

        assert_eq!(SIMPLE_FLAG.load(Ordering::SeqCst), 42);
    }
}
