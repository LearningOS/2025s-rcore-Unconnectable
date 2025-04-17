/// parent process fork the child process
pub fn fork(self: &Arc<Self>) -> Arc<Self> {
    // ---- access parent PCB exclusively
    let mut parent_inner = self.inner_exclusive_access();
    // copy user space(include trap context)
    // 地址空间通过复制父进程得到的
    let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
    let trap_cx_ppn = memory_set.translate(VirtAddr::from(TRAP_CONTEXT_BASE).into()).unwrap().ppn();
    // alloc a pid and a kernel stack in kernel space
    let pid_handle = pid_alloc();
    let kernel_stack = kstack_alloc();
    let kernel_stack_top = kernel_stack.get_top();
    let task_control_block = Arc::new(TaskControlBlock {
        pid: pid_handle,
        kernel_stack,
        inner: unsafe {
            UPSafeCell::new(TaskControlBlockInner {
                trap_cx_ppn,
                base_size: parent_inner.base_size,
                task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                task_status: TaskStatus::Ready,
                memory_set,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                exit_code: 0,
                heap_bottom: parent_inner.heap_bottom,
                program_brk: parent_inner.program_brk,
                priority: 16,
                stride: 0,
            })
        },
    });
    // add child
    parent_inner.children.push(task_control_block.clone());
    // modify kernel_sp in trap_cx
    // **** access child PCB exclusively
    let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
    trap_cx.kernel_sp = kernel_stack_top;
    // return
    task_control_block
    // **** release child PCB
    // ---- release parent PCB
}

pub fn exec(&self, elf_data: &[u8]) {
    // memory_set with elf program headers/trampoline/trap context/user stack
    // 生成一个全新的地址空间并直接替换进来
    // 原有地址空间生命周期结束，里面包含的全部物理页帧都会被回收
    let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
    let trap_cx_ppn = memory_set.translate(VirtAddr::from(TRAP_CONTEXT_BASE).into()).unwrap().ppn();

    // **** access current TCB exclusively
    // 更新数据状态
    let mut inner = self.inner_exclusive_access();
    // substitute memory_set
    inner.memory_set = memory_set;
    // update trap_cx ppn
    inner.trap_cx_ppn = trap_cx_ppn;
    // initialize base_size
    inner.base_size = user_sp;
    // initialize trap_cx
    // 修改新的地址空间中的 Trap 上下文
    let trap_cx = inner.get_trap_cx();
    *trap_cx = TrapContext::app_init_context(
        entry_point,
        user_sp,
        KERNEL_SPACE.exclusive_access().token(),
        self.kernel_stack.get_top(),
        trap_handler as usize
    );
    // **** release inner automatically
}
