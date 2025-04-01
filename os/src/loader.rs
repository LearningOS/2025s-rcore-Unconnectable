//! Loading user applications into memory

/// Get the total number of applications.
pub fn get_num_app() -> usize {
    //获取链接到内核内的应用的数目
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// get applications data
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    //返回第 app_id 个应用程序的二进制数据（
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app(); //获取应用总数
    let app_start = unsafe { 
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
        //raed应用的所有地址范围
    };
    assert!(app_id < num_app); //检查合法
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,  //目标应用的起始地址
            app_start[app_id + 1] - app_start[app_id], //计算长度
        )
    }
}
