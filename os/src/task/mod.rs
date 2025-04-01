//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of `PID_ALLOCATOR` allocates pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
mod context;
mod id;
mod manager;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use alloc::sync::Arc;
use crate::loader::{ get_app_data, get_num_app ,get_app_data_by_name};
use crate::mm::{ MapPermission, PageTable, VirtAddr };
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager};
use switch::__switch;
pub use task::{ TaskControlBlock, TaskStatus };

pub use context::TaskContext;
pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
pub use manager::add_task;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
    Processor,
};
/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap(); //取出当前任务

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;  
    // Change status to Ready 修改状态为ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task); //加入队列
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        panic!("All applications completed!");
    }

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie; //进程控制块中的状态修改为 僵尸进程 TaskStatus::Zombie 
    // Record exit code 传入inner 的exit_code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        //吧所有的子进程挂在 initproc_inner下面  也就是子进程的父进程是init_proc init_proc的子进程是他们
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear(); //当前进程的孩子向量清空。
    // deallocate user space
    inner.memory_set.recycle_data_pages();//前进程占用的资源进行早期回收 清空逻辑段area
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    // 因为不会回到该进程 调用schedule触发调度和任务切换
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    // the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        //get_app_data_by_name("initproc").unwrap()
        //初始化 name initproc
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
    /* 
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// count syscall fuc🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴
    pub fn count_syscall(&self, syscall_id: usize) {
        let mut inner = TASK_MANAGER.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].syscall_times[syscall_id] += 1;

        //let mut inner = self.inner.exclusive_access();
        //let current = inner.current_task;
        //inner.tasks[current].syscall_counts[syscall_id] += 1;
    }
    ///🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴
    /// Get the syscall times of the current 'Running' task 🔴🔴🔴🔴🔴🔴🔴🔴🔴🔴
    pub fn get_syscall_times(&self, syscall_id: usize) -> isize {
        let inner = TASK_MANAGER.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[syscall_id] as isize
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }*/
}

///
pub fn add_initproc() {
    //增加 task
    add_task(INITPROC.clone());
}
/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
/// 返回token
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

/// mmap syscall
pub fn mmap(_start: usize, _len: usize, _port: usize) -> isize {
    debug!(
        "x1b[31m start syscall mmap begins\nstart={:#x},_len={:#x} _port={:#x} \x1b[0m",
        _start,
        _len,
        _port
    );
    let _start_va: VirtAddr = _start.into();
    if !_start_va.aligned() {
        return -1;
    }
    if (_port & !0x7) != 0 || (_port & 0x7) == 0 {
        return -1;
    }

    let _end_va: VirtAddr = match _start.checked_add(_len) {
        Some(val) => VirtAddr(val),
        _ => {
            return -1;
        }
    };
    let _start_vpn = _start_va.floor();
    let _end_vpn = _end_va.ceil();

    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let cur = inner.current_task;
    if inner.tasks[cur].memory_set.is_overlap(_start_vpn, _end_vpn) {
        debug!("x1b[31m mmap is overlap\n\x1b[0m");
        return -1;
    }
    inner.tasks[cur].memory_set.mmap(
        _start_vpn,
        _end_vpn,
        MapPermission::from_bits_truncate((_port as u8) << 1) | MapPermission::U
    );
    return 0;
}

/// munmap syscall
pub fn munmap(start: usize, len: usize) -> isize {
    debug!("kernel: munmap: start = {:#x}, len = {:#x}", start, len);
    let start_va: VirtAddr = start.into();
    if !start_va.aligned() {
        return -1;
    }
    let start_vpn = start_va.floor();
    let end_va: VirtAddr = (start + len).into();
    let end_vpn = end_va.ceil();
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let cur = inner.current_task;
    debug!("kernel: munmap: start_vpn = {:?}, end_vpn = {:?}", start_vpn, end_vpn);
    return match inner.tasks[cur].memory_set.munmap(start_vpn, end_vpn) {
        Ok(_) => 0,
        Err(_) => -1,
    };
}

/// systrace
pub fn task_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    // 获取当前任务的用户态token
    let token = current_user_token(); //satp 寄存器的值，包含页表物理地址和模式信息
    let _page_table = PageTable::from_token(token); //临时PageTable
    //trace(_trace_request,_id,_data);
    match _trace_request {
        // 读取操作 - 读取1字节内存
        0 => {
            let is_valid_id = |id: usize, len: usize| -> bool {
                const ADD_MAX: usize = 0x8000_0000;
                const PAGE_SIZE: usize = 4096; //4KB
                //end = id + len ,end是在增加之后的实际页码,但是是左闭右开区间[start,end)
                //所以下面需要 -1
                let end = match id.checked_add(len) {
                    Some(val) => val,
                    _ => {
                        return false;
                    }
                };
                if end > ADD_MAX {
                    return false;
                }
                if len > PAGE_SIZE {
                    let strat_page = id / PAGE_SIZE;
                    let end_page = (end - 1) / PAGE_SIZE;
                    if strat_page != end_page {
                        return false;
                    }
                }
                true
            };
            //以下是判断代码
            let _vpn = VirtAddr::from(_id).floor();
            if !is_valid_id(_id, 1) {
                return -1;
            }
            let vpn = VirtAddr::from(_id).floor();

            // 检查页表项是否存在且可读 Read仅仅需要readable
            if let Some(pte) = _page_table.translate(vpn) {
                if !pte.is_valid() || !pte.readable() {
                    return -1;
                }

                // 安全读取字节
                let ppn = pte.ppn();
                let offset = VirtAddr::from(_id).page_offset();
                let byte = ppn.get_bytes_array()[offset];
                byte as isize
            } else {
                -1
            }
        }

        // 写入操作 - 写入usize数据
        1 => {
            let vpn = VirtAddr::from(_id).floor();

            // 检查页表项是否存在且可写
            if let Some(pte) = _page_table.translate(vpn) {
                if !pte.is_valid() || !pte.writable() || !pte.readable() {
                    return -1;
                }

                // 检查是否跨页
                let start = _id;
                let end = _id + core::mem::size_of::<usize>();
                if VirtAddr::from(start).floor() != VirtAddr::from(end - 1).floor() {
                    return -1; // 不支持跨页写入
                }

                // 安全写入
                let ppn = pte.ppn();
                let offset = VirtAddr::from(_id).page_offset();
                let bytes = _data.to_ne_bytes();
                ppn.get_bytes_array()[
                    offset..offset + core::mem::size_of::<usize>()
                ].copy_from_slice(&bytes);
                0
            } else {
                -1
            }
        }
        2 => {
            return TASK_MANAGER.get_syscall_times(_id);
        }
        // 无效请求
        _ => -1,
    }
}
