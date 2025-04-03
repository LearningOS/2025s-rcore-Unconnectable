//! Process management syscalls
use crate::{
    task::{ exit_current_and_run_next, suspend_current_and_run_next, TASK_MANAGER },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    //-1
    match _trace_request{
        0 =>{
            let value = unsafe { *(_id as *const u8) as isize }; // 安全读取
            value as isize
        }
        1=>{
            let ptr = _id as *mut usize;
            unsafe {
                *ptr = _data;
            }
            0
        }
        2=>{
            return TASK_MANAGER.get_syscall_id(_id);
        }
        _ =>{
            -1
        }
    }
    /* 
    if _trace_request == 0 {
        let value = unsafe { *(_id as *const u8) }; // 读取一个字节
        value as isize
    } else if _trace_request == 1 {
        let ptr = _id as *mut usize;
        unsafe {
            *ptr = _data;
        }
        return 0;
    } else if _trace_request == 2 {
        return TASK_MANAGER.get_syscall_id(_id);
    } else {
        return -1;
    }
    */
}
