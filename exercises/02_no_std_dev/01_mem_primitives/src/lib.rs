//! # no_std Memory Primitives
//!
//! In a `#![no_std]` environment, you have no standard library — only `core`.
//! These memory operation functions are the most fundamental building blocks in an OS kernel.
//! Functions like memcpy/memset in libc must be implemented by ourselves in bare-metal environments.
//!
//! ## Task
//!
//! Implement the following five functions:
//! - Only use the `core` crate, no `std`
//! - Do not call `core::ptr::copy`, `core::ptr::copy_nonoverlapping`, etc. (write your own loops)
//! - Handle edge cases correctly (n=0, overlapping memory regions, etc.)
//! - Pass all tests

// Force no_std in production; allow std in tests (cargo test framework requires it)
#![cfg_attr(not(test), no_std)]
#![allow(unused_variables)]

/// Copy `n` bytes from `src` to `dst`.
///
/// - `dst` and `src` must not overlap (use `my_memmove` for overlapping regions)
/// - Returns `dst`
///
/// # Safety
/// `dst` and `src` must each point to at least `n` bytes of valid memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // =========================
    // memcpy 的语义（本实验版本）
    // =========================
    //
    // - 从 src 连续读取 n 个字节，写入到 dst 指向的连续 n 个字节。
    // - **要求两段内存不重叠**；若重叠应使用 memmove（本文件的 my_memmove）。
    // - 返回 dst（这也是 C 标准库 memcpy 的返回值设计，便于链式调用）。
    //
    // =========================
    // 为什么这里是 unsafe？
    // =========================
    //
    // 因为 Rust 无法在编译期验证下面这些前置条件：
    // - dst 必须指向至少 n 个字节的可写有效内存
    // - src 必须指向至少 n 个字节的可读有效内存
    // - 访问期间这段内存不能被以违反别名规则的方式同时修改（数据竞争/未定义行为）
    // - 并且，**dst 与 src 不能重叠**（这一点是 memcpy 的语义要求）
    //
    // 在 OS 内核 / no_std 场景里，你经常需要提供类似 libc 的“原语函数”，
    // 这些函数天然就是以 unsafe 形式暴露给更上层（内核其他模块/汇编/其它语言）使用。

    // -------------------------
    // 边界情况：n == 0
    // -------------------------
    //
    // 0 字节拷贝按定义是“什么也不做”。即使 dst/src 为空指针也不应去解引用它们；
    // 这里提前返回可以避免任何潜在的无意义操作（并且符合测试用例预期）。
    if n == 0 {
        return dst;
    }

    // -------------------------
    // 核心实现：逐字节拷贝
    // -------------------------
    //
    // 本实验明确要求：不要调用 core::ptr::copy / copy_nonoverlapping 等现成实现，
    // 因为目标是训练你对“指针 + 循环”的最底层理解。
    //
    // 这里使用 add(i) 做指针偏移：
    // - 对 *const u8 / *mut u8 来说，add(1) 表示地址 + 1 字节（因为元素大小是 1）
    // - add(i) 在语义上要求偏移后的地址仍在同一个分配对象内（由调用者保证）
    //
    // 对每个字节：
    // - 从 src+i 读取一个 u8
    // - 写入 dst+i
    //
    // 注意：因为我们假设不重叠，所以正向从 0..n 拷贝不会覆盖尚未读取的 src 数据。
    for i in 0..n {
        // 读取：对 *const u8 解引用得到 u8
        let byte = *src.add(i);
        // 写入：对 *mut u8 解引用并赋值
        *dst.add(i) = byte;
    }

    dst
}

/// Set `n` bytes starting at `dst` to the value `c`.
///
/// Returns `dst`.
///
/// # Safety
/// `dst` must point to at least `n` bytes of valid writable memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_memset(dst: *mut u8, c: u8, n: usize) -> *mut u8 {
    // =========================
    // memset 的语义（本实验版本）
    // =========================
    //
    // - 将 dst 起始的 n 个字节全部设置为字节值 c
    // - 返回 dst（与 libc memset 一致）
    //
    // =========================
    // unsafe 前置条件
    // =========================
    // - dst 必须指向至少 n 个字节的可写有效内存
    // - 访问期间不能发生违反别名规则/数据竞争的并发写

    if n == 0 {
        return dst;
    }

    // 逐字节写入 c。这里同样禁止使用 core::ptr::write_bytes 等函数，
    // 目的是练习最基本的指针操作。
    for i in 0..n {
        *dst.add(i) = c;
    }

    dst
}

/// Copy `n` bytes from `src` to `dst`, correctly handling overlapping memory.
///
/// Returns `dst`.
///
/// # Safety
/// `dst` and `src` must each point to at least `n` bytes of valid memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // =========================
    // memmove 的语义（本实验版本）
    // =========================
    //
    // 与 memcpy 相同：复制 n 个字节 src -> dst，并返回 dst。
    // 但是 **memmove 必须正确处理 dst/src 重叠的情况**：
    //
    // 例子（重叠且 dst 在 src 后面）：
    //   src: [A B C D]
    //   dst 指向 src+1
    // 如果你从前往后拷贝：
    //   第一步把 A 写到 dst[0]，这会覆盖原来的 B，导致后续读取出错。
    // 正确做法是从后往前拷贝：
    //   先拷贝最后一个字节，再向前推进，保证读取时 src 还没被覆盖。
    //
    // 何时需要倒序？
    // - 当 dst 位于 src 区间内部的“后半段”时：
    //     dst > src 且 dst < src + n
    //   也即：两段区域发生重叠，并且目标起点在源起点之后。
    //
    // 其它情况（不重叠、或 dst 在 src 前面重叠）：
    // - 正向拷贝是安全的。

    if n == 0 {
        return dst;
    }

    // 将指针转成 usize 便于做区间比较（只比较地址大小，不做解引用）。
    let dst_addr = dst as usize;
    let src_addr = src as usize;

    // 计算区间 [src, src+n) 与 [dst, dst+n) 的相对位置。
    //
    // 注意：这里的加法可能在极端情况下溢出 usize。
    // 但在“有效内存 + n 合法”的前置条件下，n 不会大到导致地址溢出，
    // 这是 OS/底层场景常见的假设（由调用者保证）。
    let src_end = src_addr + n;

    if dst_addr > src_addr && dst_addr < src_end {
        // -------------------------
        // 情况 1：重叠且 dst 在后
        // -------------------------
        // 倒序复制：i = n-1, n-2, ..., 0
        //
        // 为什么不写 `for i in (0..n).rev()`？
        // 当然可以；这里用 while 只是把“索引递减”过程写得更直观。
        let mut i = n;
        while i != 0 {
            i -= 1;
            *dst.add(i) = *src.add(i);
        }
    } else {
        // -------------------------
        // 情况 2：不需要倒序
        // -------------------------
        // - 不重叠：随便怎么拷都行；我们选择正向
        // - 重叠但 dst 在 src 前：正向写不会覆盖尚未读取的 src
        for i in 0..n {
            *dst.add(i) = *src.add(i);
        }
    }

    dst
}

/// Return the length of a null-terminated byte string, excluding the trailing null.
///
/// # Safety
/// `s` must point to a valid null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_strlen(s: *const u8) -> usize {
    // =========================
    // strlen 的语义（本实验版本）
    // =========================
    //
    // - 输入是一个以 '\0'（也就是 0u8）结尾的“C 风格字节串”
    // - 返回不包含结尾 '\0' 的长度
    //
    // 例如：
    //   b"hello\0" -> 5
    //   b"\0"      -> 0
    //
    // =========================
    // unsafe 前置条件
    // =========================
    // - s 必须指向一个有效的、最终会遇到 0u8 终止符的内存序列
    // - 若缺少终止符，循环会越界读取，产生未定义行为（UB）

    // 从 0 开始逐字节扫描，直到遇到 0。
    let mut len: usize = 0;
    loop {
        // 读取当前位置的字节
        let byte = *s.add(len);
        if byte == 0 {
            break;
        }
        len += 1;
    }
    len
}

/// Compare two null-terminated byte strings.
///
/// Returns:
/// - `0`  : strings are equal
/// - `< 0`: `s1` is lexicographically less than `s2`
/// - `> 0`: `s1` is lexicographically greater than `s2`
///
/// # Safety
/// `s1` and `s2` must each point to a valid null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_strcmp(s1: *const u8, s2: *const u8) -> i32 {
    // =========================
    // strcmp 的语义（本实验版本）
    // =========================
    //
    // 比较两个以 '\0' 结尾的字节串（C 风格字符串）。
    //
    // 返回值约定（与 libc strcmp 一致的“符号”语义）：
    // - 0   : 完全相等
    // - < 0 : s1 在字典序上小于 s2
    // - > 0 : s1 在字典序上大于 s2
    //
    // “字典序”规则是按字节逐个比较：
    // - 找到第一个不同的字节时，比较它们的大小
    // - 如果一直相同直到某个字符串先结束（遇到 '\0'），短的更小
    //
    // =========================
    // unsafe 前置条件
    // =========================
    // - s1、s2 都必须是有效的、以 0u8 终止的字节序列
    // - 缺少终止符会导致越界读取（UB）

    let mut i: usize = 0;
    loop {
        // 逐字节读取
        let a = *s1.add(i);
        let b = *s2.add(i);

        // 若字节不同，直接返回差值的符号（这里返回 a-b 的 i32）
        // 注意：这里把 u8 转为 i32 再相减，避免 u8 下溢。
        if a != b {
            return (a as i32) - (b as i32);
        }

        // 若相同且已经到达 '\0'，说明两个字符串同时结束 => 相等
        if a == 0 {
            return 0;
        }

        i += 1;
    }
}

// ============================================================
// Tests (std is available under #[cfg(test)])
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcpy_basic() {
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        unsafe { my_memcpy(dst.as_mut_ptr(), src.as_ptr(), 5) };
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memcpy_zero_len() {
        let src = [0xFFu8; 4];
        let mut dst = [0u8; 4];
        unsafe { my_memcpy(dst.as_mut_ptr(), src.as_ptr(), 0) };
        assert_eq!(dst, [0u8; 4]);
    }

    #[test]
    fn test_memset_basic() {
        let mut buf = [0u8; 8];
        unsafe { my_memset(buf.as_mut_ptr(), 0xAB, 8) };
        assert!(buf.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn test_memset_partial() {
        let mut buf = [0u8; 8];
        unsafe { my_memset(buf.as_mut_ptr(), 0xFF, 4) };
        assert_eq!(&buf[..4], &[0xFF; 4]);
        assert_eq!(&buf[4..], &[0x00; 4]);
    }

    #[test]
    fn test_memmove_no_overlap() {
        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        unsafe { my_memmove(dst.as_mut_ptr(), src.as_ptr(), 4) };
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memmove_overlap_forward() {
        // Copy buf[0..4] to buf[1..5], shifting right by 1
        let mut buf = [1u8, 2, 3, 4, 5];
        unsafe { my_memmove(buf.as_mut_ptr().add(1), buf.as_ptr(), 4) };
        assert_eq!(buf, [1, 1, 2, 3, 4]);
    }

    #[test]
    fn test_strlen_basic() {
        let s = b"hello\0";
        assert_eq!(unsafe { my_strlen(s.as_ptr()) }, 5);
    }

    #[test]
    fn test_strlen_empty() {
        let s = b"\0";
        assert_eq!(unsafe { my_strlen(s.as_ptr()) }, 0);
    }

    #[test]
    fn test_strcmp_equal() {
        let a = b"hello\0";
        let b = b"hello\0";
        assert_eq!(unsafe { my_strcmp(a.as_ptr(), b.as_ptr()) }, 0);
    }

    #[test]
    fn test_strcmp_less() {
        let a = b"abc\0";
        let b = b"abd\0";
        assert!(unsafe { my_strcmp(a.as_ptr(), b.as_ptr()) } < 0);
    }

    #[test]
    fn test_strcmp_greater() {
        let a = b"abd\0";
        let b = b"abc\0";
        assert!(unsafe { my_strcmp(a.as_ptr(), b.as_ptr()) } > 0);
    }
}
