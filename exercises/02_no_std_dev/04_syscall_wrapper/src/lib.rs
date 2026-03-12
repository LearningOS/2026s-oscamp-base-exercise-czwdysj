//! # Cross-Architecture Syscall ABI Description and Wrapper
//!
//! Describe the syscall ABI for x86_64, aarch64, and riscv64 on Linux by filling in struct fields.
//! Also implement real syscall invocations on the current platform via conditional compilation.
//!
//! ## Background
//!
//! Different CPU architectures use different instructions and registers to trigger system calls:
//!
//! | Arch     | Instruction | Syscall ID Reg | Return Reg | Argument Registers              |
//! |----------|-------------|----------------|------------|---------------------------------|
//! | x86_64   | `syscall`   | rax            | rax        | rdi, rsi, rdx, r10, r8, r9     |
//! | aarch64  | `svc #0`    | x8             | x0         | x0, x1, x2, x3, x4, x5        |
//! | riscv64  | `ecall`     | a7             | a0         | a0, a1, a2, a3, a4, a5         |
//!
//! ## Task
//!
//! 1. Implement `x86_64_abi()`, `aarch64_abi()`, `riscv64_abi()` — return structs describing each arch's ABI
//! 2. (Conditional compilation) Implement real `syscall3` inline assembly on the current platform
//! 3. Build `sys_write` / `sys_read` / `sys_close` / `sys_exit` on top of `syscall3`
//!
//! ## Hints
//!
//! - Linux syscall numbers differ across architectures; x86_64 vs aarch64/riscv64 are quite different
//! - The x86_64 `syscall` instruction clobbers the rcx and r11 registers
//! - aarch64 and riscv64 share the unified syscall number table (from asm-generic)

#![cfg_attr(not(test), no_std)]

// 使用内联汇编需要从 `core::arch` 引入 `asm!` 宏。
// 注意：具体的汇编实现通过 `cfg` 做了平台限定，其他平台上不会真正生成这些指令。
use core::arch::asm;

/// Describes a Linux Syscall ABI for a specific architecture
pub struct SyscallABI {
    /// Architecture name: "x86_64", "aarch64", "riscv64"
    pub arch: &'static str,
    /// Instruction that triggers the syscall: "syscall", "svc #0", "ecall"
    pub instruction: &'static str,
    /// Register holding the syscall number
    pub id_reg: &'static str,
    /// Register holding the return value
    pub ret_reg: &'static str,
    /// Argument registers (in order)
    pub arg_regs: &'static [&'static str],
    /// Registers additionally clobbered by the syscall instruction
    pub clobbered: &'static [&'static str],
    /// write syscall number
    pub sys_write: usize,
    /// read syscall number
    pub sys_read: usize,
    /// close syscall number
    pub sys_close: usize,
    /// exit syscall number
    pub sys_exit: usize,
}

// 这个结构体 `SyscallABI` 用来**纯粹描述**某个架构下 Linux 系统调用 ABI 的约定：
// - `arch`：架构名字字符串（测试里会断言是 "x86_64" / "aarch64" / "riscv64"）
// - `instruction`：触发 syscall 的指令名字（"syscall" / "svc #0" / "ecall"）
// - `id_reg`：放“系统调用号”的寄存器名
// - `ret_reg`：放“返回值”的寄存器名
// - `arg_regs`：依次放 6 个参数的寄存器名数组
// - `clobbered`：执行 syscall 指令时，会被**额外破坏**的寄存器（x86_64 会额外 clobber rcx、r11）
// - `sys_write/sys_read/sys_close/sys_exit`：该架构下 Linux 这 4 个系统调用的编号
//
// 你的任务之一，是在 `x86_64_abi` / `aarch64_abi` / `riscv64_abi` 里**构造这个结构体**，
// 让测试中关于寄存器、指令名和 syscall 号的断言全部通过。

/// Return the x86_64 Linux syscall ABI description
pub fn x86_64_abi() -> SyscallABI {
    // TODO: Fill in the x86_64 syscall ABI
    // Hint: x86_64 uses the "syscall" instruction, syscall number in rax
    //
    // 这里需要你根据文件顶部的表格，以及测试里的期望，填充一个 `SyscallABI`：
    // - `arch` 应为 "x86_64"
    // - `instruction` 为 "syscall"
    // - `id_reg` / `ret_reg` 都是 "rax"
    // - `arg_regs` 顺序严格是：["rdi", "rsi", "rdx", "r10", "r8", "r9"]
    // - `clobbered` 至少包含 "rcx" 和 "r11"
    // - 4 个 syscall 号在测试中已经给出：write=1, read=0, close=3, exit=60
    //
    // 注意：这个函数**只是返回一个描述结构体**，不会真的发起 syscall。
    SyscallABI {
        arch: "x86_64",
        instruction: "syscall",
        id_reg: "rax",
        ret_reg: "rax",
        arg_regs: &["rdi", "rsi", "rdx", "r10", "r8", "r9"],
        clobbered: &["rcx", "r11"],
        sys_write: 1,
        sys_read: 0,
        sys_close: 3,
        sys_exit: 60,
    }
}

/// Return the aarch64 Linux syscall ABI description
pub fn aarch64_abi() -> SyscallABI {
    // TODO: Fill in the aarch64 syscall ABI
    // Hint: aarch64 uses the "svc #0" instruction, syscall number in x8
    //
    // 你需要同样构造一个 `SyscallABI`：
    // - `arch` = "aarch64"
    // - `instruction` = "svc #0"
    // - `id_reg` = "x8"（aarch64 把 syscall 号放在 x8）
    // - `ret_reg` = "x0"
    // - `arg_regs` 顺序为：["x0", "x1", "x2", "x3", "x4", "x5"]
    // - `clobbered` 在本练习中按测试期望为空数组（`is_empty()` 为 true）
    // - syscall 号：write=64, read=63, close=57, exit=93（测试中已给出）
    //
    // 同样，这里只是“描述” ABI，不执行实际 syscall。
    SyscallABI {
        arch: "aarch64",
        instruction: "svc #0",
        id_reg: "x8",
        ret_reg: "x0",
        arg_regs: &["x0", "x1", "x2", "x3", "x4", "x5"],
        clobbered: &[],
        sys_write: 64,
        sys_read: 63,
        sys_close: 57,
        sys_exit: 93,
    }
}

/// Return the riscv64 Linux syscall ABI description
pub fn riscv64_abi() -> SyscallABI {
    // TODO: Fill in the riscv64 syscall ABI
    // Hint: riscv64 uses the "ecall" instruction, syscall number in a7
    //
    // riscv64 与 aarch64 共享 asm-generic 的 syscall 号表，所以 4 个 syscall 号应与 aarch64 相同：
    // write=64, read=63, close=57, exit=93。
    // 其余字段：
    // - `arch` = "riscv64"
    // - `instruction` = "ecall"
    // - `id_reg` = "a7"
    // - `ret_reg` = "a0"
    // - `arg_regs` 顺序为：["a0", "a1", "a2", "a3", "a4", "a5"]
    // - `clobbered` 按测试期望同样为空
    SyscallABI {
        arch: "riscv64",
        instruction: "ecall",
        id_reg: "a7",
        ret_reg: "a0",
        arg_regs: &["a0", "a1", "a2", "a3", "a4", "a5"],
        clobbered: &[],
        sys_write: 64,
        sys_read: 63,
        sys_close: 57,
        sys_exit: 93,
    }
}

// ============================================================
// Real syscall implementation (conditional compilation, only active on matching platform)
// ============================================================

/// Issue a Linux syscall with up to 3 arguments.
///
/// # Safety
/// The caller must ensure the syscall number and arguments are valid.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub unsafe fn syscall3(id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    // TODO: Implement x86_64 syscall using core::arch::asm!
    // Hints:
    //   - "syscall" instruction
    //   - inlateout("rax") id => ret
    //   - in("rdi") arg0, in("rsi") arg1, in("rdx") arg2
    //   - out("rcx") _, out("r11") _
    //
    // 这个函数是“真实版”的 syscall 封装，仅在 **x86_64 Linux 平台**下编译生效：
    // - `id`：系统调用号（放到 rax）
    // - `arg0` / `arg1` / `arg2`：前三个参数，分别放入 rdi、rsi、rdx
    // - 使用 `core::arch::asm!` 发出一条 "syscall" 指令
    // - 返回值来自 rax，按 Linux 约定：若为负数则表示出错（-errno）
    //
    // 你需要：
    // 1. `use core::arch::asm;`
    // 2. 写一个 `let ret: isize;`，然后在 `asm!` 里用 inlateout("rax") 把 id 带入、把 ret 带出
    // 3. 按提示标注输入寄存器和被 clobber 的 rcx、r11
    // 4. 最后返回 ret
    //
    // 注意：这里不对返回值做任何解释，直接按 `isize` 原样返回，
    // 由调用方根据 Linux 约定（负数代表错误）自行判断。
    let mut rax = id;
    asm!(
        "syscall",
        inlateout("rax") rax,
        in("rdi") arg0,
        in("rsi") arg1,
        in("rdx") arg2,
        out("rcx") _,
        out("r11") _,
        options(nostack, preserves_flags),
    );
    rax as isize
}

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
pub unsafe fn syscall3(id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    // TODO: Implement aarch64 syscall using core::arch::asm!
    // Hints:
    //   - "svc #0" instruction
    //   - in("x8") id
    //   - inlateout("x0") arg0 => ret
    //   - in("x1") arg1, in("x2") arg2
    //
    // 这个版本仅在 **aarch64 Linux** 下生效：
    // - syscall 号放在 x8
    // - 第一个参数放 x0，并且作为返回寄存器（inlateout）
    // - 第二、第三参数放 x1、x2
    // - 通过 "svc #0" 指令触发内核
    //
    // 与 x86_64 版类似，你需要用 `core::arch::asm!` 按上述寄存器约定编码汇编，并把 x0 的值作为 ret 返回。
    let mut x0 = arg0;
    asm!(
        "svc #0",
        in("x8") id,
        inlateout("x0") x0,
        in("x1") arg1,
        in("x2") arg2,
        options(nostack),
    );
    x0 as isize
}

// Non-Linux platforms: provide a stub so the code compiles
#[cfg(not(target_os = "linux"))]
pub unsafe fn syscall3(_id: usize, _arg0: usize, _arg1: usize, _arg2: usize) -> isize {
    panic!("syscall3 is only available on Linux")
}

// Platform-specific write syscall number
#[cfg(target_arch = "x86_64")]
const NATIVE_SYS_WRITE: usize = 1;
#[cfg(target_arch = "x86_64")]
const NATIVE_SYS_READ: usize = 0;
#[cfg(target_arch = "x86_64")]
const NATIVE_SYS_CLOSE: usize = 3;
#[cfg(target_arch = "x86_64")]
const NATIVE_SYS_EXIT: usize = 60;

#[cfg(target_arch = "aarch64")]
const NATIVE_SYS_WRITE: usize = 64;
#[cfg(target_arch = "aarch64")]
const NATIVE_SYS_READ: usize = 63;
#[cfg(target_arch = "aarch64")]
const NATIVE_SYS_CLOSE: usize = 57;
#[cfg(target_arch = "aarch64")]
const NATIVE_SYS_EXIT: usize = 93;

// Fallback for other architectures (not actually used, just for compilation)
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE_SYS_WRITE: usize = 0;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE_SYS_READ: usize = 0;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE_SYS_CLOSE: usize = 0;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const NATIVE_SYS_EXIT: usize = 0;

/// Write data from `buf` to file descriptor `fd`.
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    // TODO: Call syscall3 to implement write
    //
    // 这个函数是一个“跨架构统一接口”的 write 封装：
    // - `fd`：文件描述符（0=stdin, 1=stdout, 2=stderr, 其他为普通 fd）
    // - `buf`：要写出的字节切片
    // - 返回值：与 Linux write 一样，成功时返回写入字节数（>=0），失败时返回 -errno（<0）
    //
    // 实现思路：
    // 1. 取出当前平台的 write 系统调用号：`NATIVE_SYS_WRITE`
    // 2. 将 `buf.as_ptr()` 转成 `usize` 作为第 2 个参数（arg1）
    // 3. 将 `buf.len()` 作为第 3 个参数（arg2）
    // 4. 调用 `unsafe { syscall3(NATIVE_SYS_WRITE, fd, buf.as_ptr() as usize, buf.len() as usize) }`
    //
    // 注意：
    // - 这里只需要处理最多 3 个参数的简单 syscall，因此统一用 syscall3 即可。
    // - 该函数本身是 safe 的，内部调用 unsafe syscall3 需要用 unsafe 块包起来。
    unsafe {
        syscall3(
            NATIVE_SYS_WRITE,
            fd,
            buf.as_ptr() as usize,
            buf.len() as usize,
        )
    }
}

/// Read data from file descriptor `fd` into `buf`.
pub fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    // TODO: Call syscall3 to implement read
    //
    // 这是 read 的跨架构封装：
    // - 从 `fd` 读取最多 `buf.len()` 字节到 `buf` 中
    // - 返回值为实际读取到的字节数（>=0），或错误码的相反数（<0）
    //
    // 实现思路与 sys_write 类似：
    // 1. 使用 `NATIVE_SYS_READ` 作为 syscall 号
    // 2. 第 2 个参数为缓冲区指针 `buf.as_mut_ptr() as usize`
    // 3. 第 3 个参数为缓冲区长度 `buf.len() as usize`
    // 4. 调用 `unsafe { syscall3(...) }` 得到返回值
    unsafe {
        syscall3(
            NATIVE_SYS_READ,
            fd,
            buf.as_mut_ptr() as usize,
            buf.len() as usize,
        )
    }
}

/// Close file descriptor `fd`.
pub fn sys_close(fd: usize) -> isize {
    // TODO: Call syscall3 to implement close
    //
    // 关闭一个文件描述符：
    // - 只需要 1 个参数：`fd`
    // - 其他两个参数可以传 0（不会被内核使用）
    //
    // 实现思路：
    //   `unsafe { syscall3(NATIVE_SYS_CLOSE, fd, 0, 0) }`
    //
    // 返回值约定：
    // - 0 或 >=0：成功
    // - 负数：失败（-errno）
    unsafe { syscall3(NATIVE_SYS_CLOSE, fd, 0, 0) }
}

/// Terminate the current process.
pub fn sys_exit(code: i32) -> ! {
    // TODO: Call syscall3 to implement exit
    //
    // 进程退出：
    // - `code`：退出码，按约定通常 0 表示成功，非 0 表示错误
    // - 这个函数的返回类型是 `!`（never type），表示**永远不会返回**：
    //   调用成功的话，当前进程会被内核终结。
    //
    // 实现思路（伪代码）：
    //   unsafe {
    //       syscall3(NATIVE_SYS_EXIT, code as usize, 0, 0);
    //       core::hint::unreachable_unchecked();
    //   }
    //
    // 但本实验的重点在于理解如何使用 syscall3 和 NATIVE_SYS_EXIT。
    // 你只需要保证在 Linux 测试下，调用 sys_exit 会触发正确的 exit 系统调用。
    unsafe {
        let _ = syscall3(NATIVE_SYS_EXIT, code as usize, 0, 0);
        // 按类型签名，这个函数不应返回；若 syscall 失败或返回，我们标记为不可达。
        core::hint::unreachable_unchecked();
    }
}

// ============================================================
// Tests
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    // ---- ABI knowledge tests (run on any platform) ----

    #[test]
    fn test_x86_64_instruction() {
        let abi = x86_64_abi();
        assert_eq!(abi.arch, "x86_64");
        assert_eq!(abi.instruction, "syscall");
    }

    #[test]
    fn test_x86_64_registers() {
        let abi = x86_64_abi();
        assert_eq!(abi.id_reg, "rax");
        assert_eq!(abi.ret_reg, "rax");
        assert_eq!(
            abi.arg_regs,
            &["rdi", "rsi", "rdx", "r10", "r8", "r9"],
            "x86_64 argument register order is incorrect"
        );
    }

    #[test]
    fn test_x86_64_clobbered() {
        let abi = x86_64_abi();
        assert!(
            abi.clobbered.contains(&"rcx") && abi.clobbered.contains(&"r11"),
            "x86_64 syscall clobbers rcx and r11"
        );
    }

    #[test]
    fn test_x86_64_syscall_numbers() {
        let abi = x86_64_abi();
        assert_eq!(abi.sys_write, 1);
        assert_eq!(abi.sys_read, 0);
        assert_eq!(abi.sys_close, 3);
        assert_eq!(abi.sys_exit, 60);
    }

    #[test]
    fn test_aarch64_instruction() {
        let abi = aarch64_abi();
        assert_eq!(abi.arch, "aarch64");
        assert_eq!(abi.instruction, "svc #0");
    }

    #[test]
    fn test_aarch64_registers() {
        let abi = aarch64_abi();
        assert_eq!(abi.id_reg, "x8");
        assert_eq!(abi.ret_reg, "x0");
        assert_eq!(
            abi.arg_regs,
            &["x0", "x1", "x2", "x3", "x4", "x5"],
            "aarch64 argument register order is incorrect"
        );
    }

    #[test]
    fn test_aarch64_clobbered() {
        let abi = aarch64_abi();
        assert!(
            abi.clobbered.is_empty(),
            "aarch64 svc does not clobber additional registers"
        );
    }

    #[test]
    fn test_aarch64_syscall_numbers() {
        let abi = aarch64_abi();
        assert_eq!(abi.sys_write, 64);
        assert_eq!(abi.sys_read, 63);
        assert_eq!(abi.sys_close, 57);
        assert_eq!(abi.sys_exit, 93);
    }

    #[test]
    fn test_riscv64_instruction() {
        let abi = riscv64_abi();
        assert_eq!(abi.arch, "riscv64");
        assert_eq!(abi.instruction, "ecall");
    }

    #[test]
    fn test_riscv64_registers() {
        let abi = riscv64_abi();
        assert_eq!(abi.id_reg, "a7");
        assert_eq!(abi.ret_reg, "a0");
        assert_eq!(
            abi.arg_regs,
            &["a0", "a1", "a2", "a3", "a4", "a5"],
            "riscv64 argument register order is incorrect"
        );
    }

    #[test]
    fn test_riscv64_clobbered() {
        let abi = riscv64_abi();
        assert!(
            abi.clobbered.is_empty(),
            "riscv64 ecall does not clobber additional registers"
        );
    }

    #[test]
    fn test_riscv64_syscall_numbers() {
        let abi = riscv64_abi();
        assert_eq!(abi.sys_write, 64);
        assert_eq!(abi.sys_read, 63);
        assert_eq!(abi.sys_close, 57);
        assert_eq!(abi.sys_exit, 93);
    }

    #[test]
    fn test_aarch64_riscv64_share_numbers() {
        let aarch64 = aarch64_abi();
        let riscv64 = riscv64_abi();
        assert_eq!(
            aarch64.sys_write, riscv64.sys_write,
            "aarch64 and riscv64 share asm-generic syscall numbers"
        );
        assert_eq!(aarch64.sys_read, riscv64.sys_read);
        assert_eq!(aarch64.sys_close, riscv64.sys_close);
        assert_eq!(aarch64.sys_exit, riscv64.sys_exit);
    }

    // ---- Real syscall tests (only run on Linux) ----

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;

        #[test]
        fn test_sys_write_stdout() {
            let msg = b"[syscall_wrapper] sys_write test\n";
            let ret = sys_write(1, msg);
            assert_eq!(
                ret,
                msg.len() as isize,
                "sys_write should return bytes written"
            );
        }

        #[test]
        fn test_sys_write_stderr() {
            let msg = b"[syscall_wrapper] stderr test\n";
            let ret = sys_write(2, msg);
            assert_eq!(ret, msg.len() as isize);
        }

        #[test]
        fn test_sys_write_invalid_fd() {
            let ret = sys_write(999, b"hello");
            assert!(ret < 0, "invalid fd should return negative, got {ret}");
        }

        #[test]
        fn test_sys_close_invalid_fd() {
            let ret = sys_close(999);
            assert!(ret < 0, "closing invalid fd should return negative");
        }
    }
}
