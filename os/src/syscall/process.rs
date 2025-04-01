//! Process management syscalls
/* 
use crate::console::print;
use core::mem::offset_of; 
*/
/* use crate::{
    mm::{PTEFlags,frame_alloc,VirtPageNum,PageTable,VirtAddr};
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next ,TASK_MANAGER,
        current_user_token,mmap,munmap
    };

    timer::get_time_us;
}; */
//use crate::mm::{ PTEFlags, frame_alloc, VirtPageNum, PageTable, VirtAddr };
use crate::mm::{ PageTable, VirtAddr };
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, mmap, munmap, suspend_current_and_run_next, task_trace, //TASK_MANAGER
};

use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
/// /// 提示：你可能需要通过虚拟内存管理重新实现它
/// 提示：如果 [`TimeVal`] 结构被跨页分割会怎样？
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    //ts 时间结构 time structure 这里是TimeVal
    //tz 时区 timezone 未使用
    trace!("kernel: sys_get_time");
    let token = current_user_token();
    let _page_table = PageTable::from_token(token);

    // 将时间转换为虚拟地址
    let start: usize = _ts as usize;
    let len: usize = core::mem::size_of::<TimeVal>(); // TimeVal 的大小

    //防止end溢出
    let end: usize = match start.checked_add(len) {
        Some(val) => val,
        _ => {
            return -1;
        }
    };

    // 检查是否跨页
    let start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(end).floor();
    if start_vpn != end_vpn {
        return -1;
    }

    // 翻译虚拟地址并检查权限
    // 检查页表项是否存在且可读 Read仅仅需要readable

    //这里如果获取成功,就已经定义了pte这个变量
    if let Some(pte) = _page_table.translate(start_vpn) {
        if !pte.is_valid() || !pte.writable() {
            return -1;
        }

        // 获取当前时间（以微秒为单位）
        let time_ = get_time_us();

        // 将微秒转换为秒和微秒
        let curr_time = TimeVal {
            sec: time_ / 1_000_000,
            usec: time_ % 1_000_000,
        };
        let ppn = pte.ppn();
        let offset = VirtAddr::from(start).page_offset();

        //time_bytes是一个切片
        let time_bytes = unsafe {
            core::slice::from_raw_parts(
                //裸指针
                &curr_time as *const TimeVal as *const u8,
                //size大小
                len
            )
        };
        //把ppn返回物理地址,然后取出从[offset,offset+len]的范围,然后将源切片的数据复制到目标切片
        ppn.get_bytes_array()[offset..offset + len].copy_from_slice(time_bytes);
        // 写入时间到物理内存
        0 // 成功
    } else {
        -1 // 页面不存在
    }
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// /// 提示：你可能需要通过虚拟内存管理重新实现它
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
     trace!("kernel: sys_trace");
     task_trace(_trace_request, _id, _data)
    
}
// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    /* const PAGE_SIZE: usize = 4096;
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    if (_port & !0x7) != 0 || (_port & 0x7) == 0 {
        return -1;
    }

    let aligned_len = ((_len + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
    let token = current_user_token();
    let mut _page_table = PageTable::from_token(token);

    let _start_vpn = VirtAddr::from(_start).floor();

    let _pages_count = aligned_len / PAGE_SIZE;

    for i in 0.._pages_count {
        let vpn = VirtPageNum(_start_vpn.0 + i);
        if let Some(pte) = _page_table.translate(vpn) {
            if pte.is_valid() {
                return -1;
            }
            // 检测到已映射
            //return 3是为了定位在那里的return错误
        }
    }
    //分配新的物理页面并映射
    for i in 0.._pages_count {
        let vpn = VirtPageNum(_start_vpn.0 + i);

        //使用frame_alloc分配物理页面
        let ppn = match frame_alloc() {
            Some(frame) => frame.ppn,
            None => {
                return -1;
            } // 物理内存不足
        };
        let flags = match _port {
            1 => PTEFlags::R | PTEFlags::U,
            2 => PTEFlags::W | PTEFlags::U,
            3 => PTEFlags::R | PTEFlags::W | PTEFlags::U,
            _ => unreachable!("_port should be 1, 2, or 3 due to prior check"),
        };
        _page_table.map(vpn, ppn, flags as PTEFlags);
    }
    0; */
    return mmap(_start, _len, _port);
}
// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    /* trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start_va = VirtAddr::from(_start);
    if !start_va.aligned() {
        return -1;
    }
    const PAGE_SIZE: usize = 4096;
    //检查参数
    if _start % PAGE_SIZE != 0 {
        println!("\x1b[31m argu Wrong\x1b[0m");
        return -1;
    }
    let aligned_len = ((_len + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
    let token = current_user_token();
    let mut _page_table = PageTable::from_token(token);

    let _start_vpn = VirtAddr::from(_start).floor();
    let _pages_count = aligned_len / PAGE_SIZE;

    for i in 0.._pages_count {
        let vpn = VirtPageNum(_start_vpn.0 + i);
        match _page_table.translate(vpn) {
            //Some(_pte) => (),
            //_ => return -1,
            /*  if pte.is_valid(){
                _page_table.unmap(vpn);
            } */
            Some(pte) => {
                if !pte.is_valid() {
                    //println!("\x1b[31m vpn {:?} is invalid 页面存在但无效 \x1b[0m", vpn);
                    return -1; // 页面存在但无效
                }
            }
            None => {
                //println!("\x1b[31m vpn {:?} not mapped \x1b[0m", vpn);
                return -1;
            }
        }
    }

    for i in 0.._pages_count {
        let vpn = VirtPageNum(_start_vpn.0 + i);
        _page_table.unmap(vpn);
    }

    0 */
    return munmap(_start, _len);
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
