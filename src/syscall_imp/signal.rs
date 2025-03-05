use core::ffi::c_void;

pub(crate) fn sys_rt_sigprocmask(
    _how: i32,
    _set: *const c_void,
    _oldset: *mut c_void,
    _sigsetsize: usize,
) -> isize {
    warn!("sys_rt_sigprocmask: not implemented");
    0
}

pub(crate) fn sys_rt_sigaction(
    _signum: i32,
    _act: *const c_void,
    _oldact: *mut c_void,
    _sigsetsize: usize,
) -> isize {
    warn!("sys_rt_sigaction: not implemented");
    0
}
