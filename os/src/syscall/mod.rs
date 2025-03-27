//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

/// write syscall
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// trace syscall
const SYSCALL_TRACE: usize = 410;

mod fs;
mod process;

use fs::*;
use process::*;

use crate::task::TASK_MANAGER;
/// const for 
pub const SYSCALL_RESET_COUNTS: usize = 999;
/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    TASK_MANAGER.add_syscall_count(syscall_id); // 更新计数
    //static mut WRITE_COUNT: usize = 0; // 静态变量记录 SYSCALL_WRITE 调用次数
    /*if syscall_id == SYSCALL_EXIT { // 只检查 SYSCALL_WRITE
        unsafe {
            WRITE_COUNT += 1;
            println!(
                "\x1b[31mSYSCALL_WRITE (id: 64) called #{}, args: {:?}\x1b[0m",
                WRITE_COUNT, args
            );
        }
    }*/
    match syscall_id {
        SYSCALL_RESET_COUNTS => {
            TASK_MANAGER.reset_syscall_counts();
            0
        }
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TRACE => sys_trace(args[0], args[1], args[2]),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
