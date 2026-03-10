//! # Process and Pipes
//!
//! In this exercise, you will learn how to create child processes and communicate through pipes.
//!
//! ## Concepts
//! - `std::process::Command` creates child processes (corresponds to `fork()` + `execve()` system calls)
//! - `Stdio::piped()` sets up pipes (corresponds to `pipe()` + `dup2()` system calls)
//! - Communicate with child processes via stdin/stdout
//! - Obtain child process exit status (corresponds to `waitpid()` system call)
//!
//! ## OS Concepts Mapping
//! This exercise demonstrates user‑space abstractions over underlying OS primitives:
//! - **Process creation**: Rust's `Command::new()` internally invokes `fork()` to create a child process,
//!   then `execve()` (or equivalent) to replace the child's memory image with the target program.
//! - **Inter‑process communication (IPC)**: Pipes are kernel‑managed buffers that allow one‑way data
//!   flow between related processes. The `pipe()` system call creates a pipe, returning two file
//!   descriptors (read end, write end). `dup2()` duplicates a file descriptor, enabling redirection
//!   of standard input/output.
//! - **Resource management**: File descriptors (including pipe ends) are automatically closed when
//!   their Rust `Stdio` objects are dropped, preventing resource leaks.
//!
//! ## Exercise Structure
//! 1. **Basic command execution** (`run_command`) – launch a child process and capture its stdout.
//! 2. **Bidirectional pipe communication** (`pipe_through_cat`) – send data to a child process (`cat`)
//!    and read its output.
//! 3. **Exit code retrieval** (`get_exit_code`) – obtain the termination status of a child process.
//! 4. **Advanced: error‑handling version** (`run_command_with_result`) – learn proper error propagation.
//! 5. **Advanced: complex bidirectional communication** (`pipe_through_grep`) – interact with a filter
//!    program that reads multiple lines and produces filtered output.
//!
//! Each function includes a `TODO` comment indicating where you need to write code.
//! Run `cargo test` to check your implementations.

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

/// Execute the given shell command and return its stdout output.
///
/// For example: `run_command("echo", &["hello"])` should return `"hello\n"`
///
/// # Underlying System Calls
/// - `Command::new(program)` → `fork()` + `execve()` family
/// - `Stdio::piped()` → `pipe()` + `dup2()` (sets up a pipe for stdout)
/// - `.output()` → `waitpid()` (waits for child process termination)
///
/// # Implementation Steps
/// 1. Create a `Command` with the given program and arguments.
/// 2. Set `.stdout(Stdio::piped())` to capture the child's stdout.
/// 3. Call `.output()` to execute the child and obtain its `Output`.
/// 4. Convert the `stdout` field (a `Vec<u8>`) into a `String`.
pub fn run_command(program: &str, args: &[&str]) -> String {
    // 1. 使用 Command::new 创建一个命令。
    // 这相当于在 Linux 终端里输入命令的前半部分。
    let output = Command::new(program)
        // 传入参数，比如命令是 "echo"，参数是 "hello"
        .args(args)
        // 关键点：设置 stdout 为 Stdio::piped()。
        // 这就像在命令后面加上一个隐形的管道，把原本要打印到屏幕上的内容拦截下来。
        .stdout(Stdio::piped())
        // 执行命令并等待它结束。
        // output() 会收集子进程所有的输出结果（包括 stdout 和 stderr）。
        .output()
        // 这里的 expect 是为了防止程序本身找不到命令而崩溃（比如命令名打错了）。
        .expect("Failed to execute command");

    // 2. 将字节数组 (Vec<u8>) 转换成我们能看懂的字符串 (String)。
    // 子进程输出的是原始二进制数据，我们需要把它解释为 UTF-8 编码的文本。
    String::from_utf8(output.stdout).expect("Output was not valid UTF-8")
}

pub fn pipe_through_cat(input: &str) -> String {
    // 1. 创建 cat 命令，它会原样输出它收到的内容。
    // 我们需要通过管道把内容喂给它的 stdin，并从它的 stdout 把内容接回来。
    let mut child = Command::new("cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // 注意这里用的是 spawn() 而不是 output()。
        // spawn() 会让子进程在后台跑起来，主线程可以继续跟它“说话”。
        .spawn()
        .expect("Failed to spawn cat");

    // 2. 获取子进程的 stdin 句柄。
    // take() 是为了拿到所有权，这样我们才能往里写数据。
    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        // 把我们要发送的字符串写进子进程的嘴巴里。
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
        // 💡 重要：当这个大括号结束时，stdin 变量会被自动销毁 (drop)。
        // 这会关闭管道的写入端，向子进程发送一个 EOF (文件结束标志)。
        // 没了这个，cat 就会一直等下去，以为你还有话没说完，导致程序卡死。
    }

    // 3. 从子进程的 stdout 读回内容。
    let mut output = String::new();
    child
        .stdout
        .take()
        .expect("Failed to open stdout")
        .read_to_string(&mut output)
        .expect("Failed to read stdout");

    // 4. 等待子进程彻底退出。
    child.wait().expect("Failed to wait on child");

    output
}

pub fn get_exit_code(command: &str) -> i32 {
    // 1. 使用 sh -c 来执行一整行 shell 命令。
    // 比如 command 是 "false"，对应的退出码通常是 1。
    let status = Command::new("sh")
        .args(["-c", command])
        // status() 会执行并返回退出状态信息。
        .status()
        .expect("Failed to get status");

    // 2. 提取退出码。
    // code() 返回的是 Option<i32>，如果进程是被信号强行杀死的，它可能是 None。
    status.code().unwrap_or(-1)
}

pub fn run_command_with_result(program: &str, args: &[&str]) -> io::Result<String> {
    // 1. 执行命令并捕获输出，但不使用 expect，而是使用 ? 符号向上抛出错误。
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .output()?; // 如果命令没找到，直接返回 Err(io::Error)

    // 2. 将字节转换为字符串。
    // 如果转换失败（比如输出是二进制乱码），我们手动创建一个 InvalidData 类型的错误。
    String::from_utf8(output.stdout).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn pipe_through_grep(pattern: &str, input: &str) -> String {
    // 1. 启动 grep 进程，传入过滤模式。
    let mut child = Command::new("grep")
        .arg(pattern)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn grep");

    // 2. 将输入数据写入 grep。
    {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
        // 💡 再次提醒：stdin 在这里被 drop，grep 才会知道输入结束了。
    }

    // 3. 读取过滤后的结果。
    let mut output = String::new();
    child
        .stdout
        .take()
        .expect("Failed to open stdout")
        .read_to_string(&mut output)
        .expect("Failed to read stdout");

    // 4. 收尾。
    child.wait().expect("Failed to wait on child");

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_echo() {
        let output = run_command("echo", &["hello"]);
        assert_eq!(output.trim(), "hello");
    }

    #[test]
    fn test_run_with_args() {
        let output = run_command("echo", &["-n", "no newline"]);
        assert_eq!(output, "no newline");
    }

    #[test]
    fn test_pipe_cat() {
        let output = pipe_through_cat("hello pipe!");
        assert_eq!(output, "hello pipe!");
    }

    #[test]
    fn test_pipe_multiline() {
        let input = "line1\nline2\nline3";
        assert_eq!(pipe_through_cat(input), input);
    }

    #[test]
    fn test_exit_code_success() {
        assert_eq!(get_exit_code("true"), 0);
    }

    #[test]
    fn test_exit_code_failure() {
        assert_eq!(get_exit_code("false"), 1);
    }

    #[test]
    fn test_run_command_with_result_success() {
        let result = run_command_with_result("echo", &["hello"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }

    #[test]
    fn test_run_command_with_result_nonexistent() {
        let result = run_command_with_result("nonexistent_command_xyz", &[]);
        // Should be an error because command not found
        assert!(result.is_err());
    }

    #[test]
    fn test_pipe_through_grep_basic() {
        let input = "apple\nbanana\ncherry\n";
        let output = pipe_through_grep("a", input);
        // grep outputs matching lines with newline
        assert_eq!(output, "apple\nbanana\n");
    }

    #[test]
    fn test_pipe_through_grep_no_match() {
        let input = "apple\nbanana\ncherry\n";
        let output = pipe_through_grep("z", input);
        // No lines match -> empty string
        assert_eq!(output, "");
    }

    #[test]
    fn test_pipe_through_grep_multiline() {
        let input = "first line\nsecond line\nthird line\n";
        let output = pipe_through_grep("second", input);
        assert_eq!(output, "second line\n");
    }
}
